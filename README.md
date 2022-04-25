# Retriever

## Background
The Retriever is one of the timeSol microservices, responsible for the collection of NFT data from MagicEden, the most popular Solana NFT marketplace. The data collected will be useful for NFT buyers who can utilize this data to see trends about a particular collection.

## How to Run Retriever

### Create a database called 'magiceden' in Postgres
You can change the name if you'd like, but the code uses this database name in the code. I'll probably change this to depend upon Cargo.toml in the future.

Set environment variables in the service

* config_path -- Path to DB credentials
* trace_path -- Set location to where trace files should be saved

The DB credentials configuration is in YAML format:

```
user: <postgres-username>
password: <postgres-password>
host: <postgres-server-address>
dbname: magiceden
```
### Add service to systemd
* `cp retriever.service /etc/systemd/system/`
### Enable the service at startup
* `sudo systemctl enable retriever.service`
### Start the service
* `sudo systemctl start retriever.service`
### Check service status for any errors
* `systemctl status retriever.service`

You should start seeing data in your database if everything ran correctly.
