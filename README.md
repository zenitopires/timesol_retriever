# Retriever

## Background
Solgraph seeks to make data about a number of Solana NFT projects easily digestible for NFT buyers.
The data provided will show you hourly/daily/weekly trends about a particular collection. 

This data is collected by the Retriever, it will make asynchronous requests to the MagicEden API and shuffle them into the PostgreSQL database. 

## How to Run Retriever

Set environment variables in the service

* config_path -- Path to DB credentials
* trace_path -- Set location to where trace files should be saved
