#[macro_use]
extern crate serde;
use candid::{Decode, Encode, Principal};
use ic_cdk::api::time;
use ic_cdk_macros::*;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Property {
    id: u64,
    address: String,
    owner_id: Principal,
    tokenized_shares: u64,
    created_at: u64,
    updated_at: Option<u64>,
    history: Vec<HistoryEntry>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct HistoryEntry {
    timestamp: u64,
    event: String,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct User {
    id: Principal,
    name: String,
    contact_info: String,
    role: String,
}

impl Storable for Property {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

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

    static USERS_STORAGE: RefCell<StableBTreeMap<Principal, User, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct PropertyPayload {
    address: String,
    tokenized_shares: u64,
}

#[query]
async fn get_property(id: u64) -> Result<Property, Error> {
    match _get_property(&id) {
        Some(property) => Ok(property),
        None => Err(Error::NotFound {
            msg: format!("A property with id={} not found", id),
        }),
    }
}

#[update]
async fn add_property(property: PropertyPayload) -> Result<Property, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());

    if user_role != "owner" {
        return Err(Error::Unauthorized { msg: "Only owners can add properties".to_string() });
    }

    if property.address.is_empty() {
        return Err(Error::InvalidInput { msg: "Address must be provided and non-empty".to_string() });
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
        owner_id: caller,
        created_at: time(),
        updated_at: None,
        history: vec![HistoryEntry { timestamp: time(), event: "Property created".to_string() }],
    };

    do_insert_property(&property);
    log_event(format!("Property added: {:?}", property));
    Ok(property)
}

#[update]
async fn update_property(id: u64, payload: PropertyPayload) -> Result<Property, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());

    if user_role != "owner" {
        return Err(Error::Unauthorized { msg: "Only owners can update properties".to_string() });
    }

    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut property) => {
            if property.owner_id != caller {
                return Err(Error::Unauthorized { msg: "Only the owner can update this property".to_string() });
            }

            if !payload.address.is_empty() {
                property.address = payload.address;
            }
            property.tokenized_shares = payload.tokenized_shares;
            property.updated_at = Some(time());
            property.history.push(HistoryEntry { timestamp: time(), event: "Property updated".to_string() });

            do_insert_property(&property);
            log_event(format!("Property updated: {:?}", property));
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
async fn delete_property(id: u64) -> Result<Property, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());

    if user_role != "admin" {
        return Err(Error::Unauthorized { msg: "Only admins can delete properties".to_string() });
    }

    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(property) => {
            log_event(format!("Property deleted: {:?}", property));
            Ok(property)
        },
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't delete a property with id={}. Property not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct UserPayload {
    name: String,
    contact_info: String,
    role: String,
}

#[query]
async fn get_user(id: Principal) -> Result<User, Error> {
    match _get_user(&id) {
        Some(user) => Ok(user),
        None => Err(Error::NotFound {
            msg: format!("A user with id={} not found", id),
        }),
    }
}

#[update]
async fn add_user(user: UserPayload) -> Result<User, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());

    if user_role != "admin" {
        return Err(Error::Unauthorized { msg: "Only admins can add users".to_string() });
    }

    if user.name.is_empty() || user.contact_info.is_empty() || user.role.is_empty() {
        return Err(Error::InvalidInput { msg: "All fields must be provided and non-empty".to_string() });
    }

    let id = caller;

    let user = User {
        id,
        name: user.name,
        contact_info: user.contact_info,
        role: user.role,
    };

    do_insert_user(&user);
    log_event(format!("User added: {:?}", user));
    Ok(user)
}

#[update]
async fn update_user(id: Principal, payload: UserPayload) -> Result<User, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());

    if user_role != "admin" {
        return Err(Error::Unauthorized { msg: "Only admins can update users".to_string() });
    }

    match USERS_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut user) => {
            if !payload.name.is_empty() {
                user.name = payload.name;
            }
            if !payload.contact_info.is_empty() {
                user.contact_info = payload.contact_info;
            }
            if !payload.role.is_empty() {
                user.role = payload.role;
            }
            do_insert_user(&user);
            log_event(format!("User updated: {:?}", user));
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
async fn delete_user(id: Principal) -> Result<User, Error> {
    let caller = ic_cdk::caller();
    let user_role = get_user_role(caller).await.unwrap_or("user".to_string());
   
    if user_role != "admin" {
        return Err(Error::Unauthorized { msg: "Only admins can delete users".to_string() });
    }

    match USERS_STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(user) => {
            log_event(format!("User deleted: {:?}", user));
            Ok(user)
        },
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't delete a user with id={}. User not found.",
                id
            ),
        }),
    }
}

#[update]
async fn transfer_ownership(property_id: u64, to: Principal, shares: u64) -> Result<Property, Error> {
    let caller = ic_cdk::caller();
    match STORAGE.with(|service| service.borrow().get(&property_id)) {
        Some(mut property) => {
            if property.owner_id != caller {
                return Err(Error::Unauthorized { msg: "Only the owner can transfer ownership".to_string() });
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
                event: format!("Transferred {} shares to {}", shares, to) 
            });

            do_insert_property(&property);
            log_event(format!("Ownership transferred: {:?}", property));
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

fn _get_user(id: &Principal) -> Option<User> {
    USERS_STORAGE.with(|service| service.borrow().get(id))
}

// Simulate fetching the user role from a user management system
async fn get_user_role(user_id: Principal) -> Option<String> {
    USERS_STORAGE.with(|service| service.borrow().get(&user_id)).map(|user| user.role)
}

// Log events using IC's event logging capabilities
fn log_event(event: String) {
    ic_cdk::println!("{}", event); // Placeholder for actual logging mechanism
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    Unauthorized { msg: String },
    NotFound { msg: String },
    InvalidInput { msg: String },
}

// Need this to generate candid
ic_cdk::export_candid!();
