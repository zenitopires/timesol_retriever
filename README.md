# Retriever

## Background
The Retriever is one of the timeSol microservices, responsible for the collection of NFT data from MagicEden, the most popular Solana NFT marketplace. The data collected will be useful for NFT buyers who can utilize this data to see trends about a particular collection.

## How to Run Retriever

Set environment variables in the service

* config_path -- Path to DB credentials
* trace_path -- Set location to where trace files should be saved

The DB credentials configuration is in YAML format
user: <postgres-username>
password: <postgres-password>
host: <postgres-server-address>
dbname: magiceden
