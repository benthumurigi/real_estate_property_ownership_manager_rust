# Tokenized Real Estate Ownership Decentralized App In Rust

The Tokenized Real Estate Ownership project is a blockchain-based platform that enables fractional ownership of real estate properties through tokenization. This project aims to democratize real estate investment by allowing investors to purchase tokenized shares of properties, thereby gaining exposure to the real estate market with lower entry barriers.

## Features

### 1. Property Management

- Create, read, update, and delete real estate properties.
- Each property includes details such as address, owner ID, tokenized shares, and a history of changes.

### 2. User Management

- Create, read, update, and delete user information.
- Each user includes details such as name and contact information.

### 3. Product History

- Maintain a history of changes for each property, including timestamps and descriptions of changes.

### 4. Ownership Transfer

- Transfer ownership shares between users securely and transparently.
- Update ownership details and record the transfer in the property history.

### 5. Input Validation

- Validate input data to ensure that required fields are provided and follow the expected format.
- Prevent invalid data from being entered into the system, improving data integrity and accuracy.

### 6. Error Handling

- Provide error handling mechanisms to gracefully handle unexpected errors or invalid requests.
- Return appropriate error messages and status codes to the client to aid in debugging and troubleshooting.

### Requirements

- rustc 1.64 or higher

```bash
$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
$ source "$HOME/.cargo/env"
```

- rust wasm32-unknown-unknown target

```bash
$ rustup target add wasm32-unknown-unknown
```

- candid-extractor

```bash
$ cargo install candid-extractor
```

- install `dfx`

```bash
$ DFX_VERSION=0.15.0 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"
$ echo 'export PATH="$PATH:$HOME/bin"' >> "$HOME/.bashrc"
$ source ~/.bashrc
$ dfx start --background --clean
```

## API Endpoints

### Property Endpoints

1. `POST /properties`: Create a new property.
2. `GET /properties/:id`: Get details of a specific property by ID.
3. `PUT /properties/:id`: Update details of a specific property by ID.
4. `DELETE /properties/:id`: Delete a specific property by ID.
5. `POST /properties/:id/transfer`: Transfer ownership of a property.

### Supplier Endpoints

1. `POST /suppliers`: Create a new user.
2. `GET /suppliers/:id`: Get details of a specific user by ID.
3. `PUT /suppliers/:id`: Update details of a specific user by ID.
4. `DELETE /suppliers/:id`: Delete a specific user by ID.

## Example API Endpoints Usage

- Create 2 new users:

```bash
dfx canister call tokenized_real_estate_ownership_rust add_user '(record { name = "User 1"; contact_info = "user1@users.com"; })'
dfx canister call tokenized_real_estate_ownership_rust add_user '(record { name = "User 2"; contact_info = "user2@users.com"; })'
```

- Read User 1's details:

```bash
dfx canister call tokenized_real_estate_ownership_rust get_user '(0)'
```

- Update User 2's contact details:

```bash
dfx canister call tokenized_real_estate_ownership_rust update_user '(1, record {name = "User 2"; contact_info = "updateduser2@user.com"; })'
```

- Create a property:

```bash
dfx canister call tokenized_real_estate_ownership_rust add_property '(record {address = "Address 1"; tokenized_shares = 1000; owner_id = 0; })'
```

- Read Property's details:

```bash
dfx canister call tokenized_real_estate_ownership_rust get_property '(<PROPERTY_ID>)'
```

- Update Property's address:

```bash
dfx canister call tokenized_real_estate_ownership_rust update_property '(<PROPERTY_ID>, record {address = "Updated Address 2"; tokenized_shares = 1000; owner_id = <USER_1_ID>; })'
```

- Transfer property from User 1 to User 2:

```bash
dfx canister call tokenized_real_estate_ownership_rust transfer_ownership '(<PROPERTY_ID>, <USER_1_ID>, <USER_2_ID>, 300)'
```

- Delete the property, then the 2 users:

```bash
dfx canister call tokenized_real_estate_ownership_rust delete_property '(<PROPERTY_ID>)'
dfx canister call tokenized_real_estate_ownership_rust delete_user '(<USER_1_ID>)'
dfx canister call tokenized_real_estate_ownership_rust delete_user '(<USER_2_ID>)'
```

PLEASE REMEMBER TO REPLACE THE VARIOUS IDs WITH THE APPROPRIATE IDs FROM THE RESULTS RETURNED FROM THE TERMINAL
