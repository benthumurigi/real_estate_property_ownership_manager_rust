#[macro_use]
extern crate serde;
use candid::{Decode, Encode, CandidType, Principal};
use ic_cdk::api::time;
use ic_cdk_macros::{query, update};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(CandidType, Clone, Serialize, Deserialize, Default)]
struct Property {
    id: u64,
    address: String,
    owner_id: u64,
    tokenized_shares: u64,
    created_at: u64,
    created_by: Principal,
    updated_at: Option<u64>,
    updated_by: Option<Principal>,
    history: Vec<HistoryEntry>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Default)]
struct HistoryEntry {
    timestamp: u64,
    event: String,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Default)]
struct User {
    id: u64,
    name: String,
    contact_info: String,
    created_at: u64,
    created_by: Principal,
    updated_at: Option<u64>,
    updated_by: Option<Principal>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Property {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Property {
    const MAX_SIZE: u32 = 2048;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for User {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for User {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Property, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static USERS_STORAGE: RefCell<StableBTreeMap<u64, User, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));
}

#[derive(CandidType, Serialize, Deserialize, Default)]
struct PropertyPayload {
    address: String,
    tokenized_shares: u64,
    owner_id: u64,
}

#[query]
fn get_property(id: u64) -> Result<Property, Error> {
    match _get_property(&id) {
        Some(property) => Ok(property),
        None => Err(Error::NotFound {
            msg: format!("A property with id={} not found", id),
        }),
    }
}

#[update]
fn add_property(property: PropertyPayload) -> Result<Property, Error> {
    if property.address.is_empty() {
        return Err(Error::InvalidInput { msg: "All fields must be provided and non-empty".to_string() });
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let property = Property {
        id,
        address: property.address,
        tokenized_shares: property.tokenized_shares,
        owner_id: property.owner_id,
        created_at: time(),
        created_by: ic_cdk::caller(), // Use the caller as the creator
        updated_at: None,
        updated_by: None,
        history: vec![HistoryEntry { timestamp: time(), event: "Property created".to_string() }],
    };

    do_insert_property(&property);
    Ok(property)
}

#[update]
fn update_property(id: u64, payload: PropertyPayload) -> Result<Property, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut property) => {
            if !payload.address.is_empty() {
                property.address = payload.address;
            }
            property.tokenized_shares = payload.tokenized_shares;
            property.owner_id = payload.owner_id;
            property.updated_at = Some(time());
            property.updated_by = Some(ic_cdk::caller()); // Use the caller as the updater
            property.history.push(HistoryEntry { timestamp: time(), event: "Property updated".to_string() });
            do_insert_property(&property);
            Ok(property)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't update a property with id={}. Property not found",
                id
            ),
        }),
    }
}

#[update]
fn delete_property(id: u64) -> Result<Property, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(property) => Ok(property),
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't delete a property with id={}. Property not found.",
                id
            ),
        }),
    }
}

#[query]
fn get_all_properties(page: u64, page_size: u64) -> Vec<Property> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .skip((page * page_size) as usize)
            .take(page_size as usize)
            .map(|(_, property)| property.clone())
            .collect()
    })
}

#[derive(CandidType, Serialize, Deserialize, Default)]
struct UserPayload {
    name: String,
    contact_info: String,
}

#[query]
fn get_user(id: u64) -> Result<User, Error> {
    match _get_user(&id) {
        Some(user) => Ok(user),
        None => Err(Error::NotFound {
            msg: format!("A user with id={} not found", id),
        }),
    }
}

#[update]
fn add_user(user: UserPayload) -> Result<User, Error> {
    if user.name.is_empty() || user.contact_info.is_empty() {
        return Err(Error::InvalidInput { msg: "All fields must be provided and non-empty".to_string() });
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let user = User {
        id,
        name: user.name,
        contact_info: user.contact_info,
        created_at: time(),
        created_by: ic_cdk::caller(), // Use the caller as the creator
        updated_at: None,
        updated_by: None,
    };

    do_insert_user(&user);
    Ok(user)
}

#[update]
fn update_user(id: u64, payload: UserPayload) -> Result<User, Error> {
    match USERS_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut user) => {
            if !payload.name.is_empty() {
                user.name = payload.name;
            }
            if !payload.contact_info.is_empty() {
                user.contact_info = payload.contact_info;
            }
            user.updated_at = Some(time());
            user.updated_by = Some(ic_cdk::caller()); // Use the caller as the updater
            do_insert_user(&user);
            Ok(user)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't update a user with id={}. User not found",
                id
            ),
        }),
    }
}

#[update]
fn delete_user(id: u64) -> Result<User, Error> {
    match USERS_STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(user) => Ok(user),
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't delete a user with id={}. User not found.",
                id
            ),
        }),
    }
}

#[query]
fn get_all_users(page: u64, page_size: u64) -> Vec<User> {
    USERS_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .skip((page * page_size) as usize)
            .take(page_size as usize)
            .map(|(_, user)| user.clone())
            .collect()
    })
}

#[update]
fn transfer_ownership(property_id: u64, from: u64, to: u64, shares: u64) -> Result<Property, Error> {
    match STORAGE.with(|service| service.borrow().get(&property_id)) {
        Some(mut property) => {
            if property.owner_id != from {
                return Err(Error::Unauthorized { msg: "The from address is not the current owner_id".to_string() });
            }
            if shares > property.tokenized_shares {
                return Err(Error::InvalidInput { msg: "Not enough shares available for transfer".to_string() });
            }

            // Transfer ownership
            property.tokenized_shares -= shares;
            if property.tokenized_shares == 0 {
                property.owner_id = to.clone();
            }

            property.history.push(HistoryEntry { 
                timestamp: time(), 
                event: format!("Transferred {} shares from {} to {}", shares, from, to) 
            });

            do_insert_property(&property);
            Ok(property)
        }
        None => Err(Error::NotFound {
            msg: format!("A property with id={} not found", property_id),
        }),
    }
}

// Helper methods to perform inserts and gets
fn do_insert_property(property: &Property) {
    STORAGE.with(|service| service.borrow_mut().insert(property.id, property.clone()));
}

fn do_insert_user(user: &User) {
    USERS_STORAGE.with(|service| service.borrow_mut().insert(user.id, user.clone()));
}

fn _get_property(id: &u64) -> Option<Property> {
    STORAGE.with(|service| service.borrow().get(id))
}

fn _get_user(id: &u64) -> Option<User> {
    USERS_STORAGE.with(|service| service.borrow().get(id))
}

#[derive(CandidType, Deserialize, Serialize)]
enum Error {
    Unauthorized { msg: String },
    NotFound { msg: String },
    InvalidInput { msg: String },
}

// need this to generate candid
ic_cdk::export_candid!();
