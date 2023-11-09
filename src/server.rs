use tonic::{transport::Server, Request, Response, Status};
use crate::key_value::key_value_storage_server::{KeyValueStorage, KeyValueStorageServer};
use crate::key_value::{KeyRequest, KeyValuePair, KeysRequest, KeysResponse};

// Load proto3 definitions
pub mod key_value {
    tonic::include_proto!("key_value");
}

// Define the structure of the key value storage
pub struct SledKeyValueStorage {
    database: sled::Db,
}

// Implement key value storage with the sled database
#[tonic::async_trait]
impl KeyValueStorage for SledKeyValueStorage {
    // Asynchronously find a value by its key in the storage.
    async fn find_by_key(&self, request: Request<KeyRequest>) -> Result<Response<KeyValuePair>, Status> {
        let key = request.into_inner().key;

        // Attempt to retrieve the value from the database using the key.
        match self.database.get(&key) {
            // If the key is found and a value is returned...
            Ok(Some(value_bytes)) => {
                let key_value_pair = KeyValuePair { key, value: value_bytes.to_vec() };
                Ok(Response::new(key_value_pair))
            }
            // If the key is not found in the database, return a "not found" error.
            Ok(None) => Err(Status::not_found("No entry found for the specified key")),
            // If there is an error accessing the database, return an internal error.
            Err(_) => Err(Status::internal("Error occurred fetching the key from the database")),
        }
    }

    // Asynchronously delete a key-value pair from the storage.
    async fn delete(&self, request: Request<KeyRequest>) -> Result<Response<KeyRequest>, Status> {
        let key_request = request.into_inner();

        // Attempt to remove the key-value pair from the database.
        match self.database.remove(&key_request.key) {
            // If successful, return the original key request in the response.
            Ok(_) => Ok(Response::new(key_request)),
            // If there is an error during deletion, return an internal error.
            Err(_) => Err(Status::internal("Error occurred deleting the key from the database")),
        }
    }

    // Asynchronously insert a key-value pair into the storage.
    async fn insert(&self, request: Request<KeyValuePair>) -> Result<Response<KeyValuePair>, Status> {
        let KeyValuePair { key, value } = request.into_inner();
        let value_clone = value.clone();

        // Attempt to insert the key-value pair into the database.
        match self.database.insert(&key, value) {
            // If successful, return the original key-value pair in the response.
            Ok(_) => {
                let inserted_pair = KeyValuePair { key, value: value_clone };
                Ok(Response::new(inserted_pair))
            }
            // If there is an error during insertion, return an internal error.
            Err(_) => Err(Status::internal("Error occurred inserting the key-value pair into the database")),
        }
    }

    // Asynchronously retrieve all keys that match a certain prefix.
    async fn keys(&self, request: Request<KeysRequest>) -> Result<Response<KeysResponse>, Status> {
        let prefix = request.into_inner().prefix;

        let mut keys = Vec::new();
        // Iterate over all key-value pairs in the database that match the prefix.
        for key_result in self.database.scan_prefix(&prefix) {
            match key_result {
                // If a key is found...
                Ok((key_bytes, _)) => {
                    // Convert the binary key (IVec) to a UTF-8 string and push it to the keys vector.
                    match String::from_utf8(key_bytes.to_vec()) {
                        Ok(key_string) => keys.push(key_string),
                        Err(_) => return Err(Status::internal("Key found is not valid UTF-8")),
                    }
                },
                // If there is an error during the database scan, return an internal error.
                Err(_) => return Err(Status::internal("Database error occurred during key scan")),
            }
        }

        // Create a response with the list of keys and send it back.
        let response = KeysResponse { keys };
        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_name = "database";
    let address = "[::1]:10522".parse()?;
    let service = SledKeyValueStorage {
        database: sled::open(database_name).unwrap()
    };
    println!("Listening on 10522...");
    Server::builder()
        .add_service(KeyValueStorageServer::new(service))
        .serve(address)
        .await?;
    Ok(())
}
