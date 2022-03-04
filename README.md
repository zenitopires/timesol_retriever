# Solgraph Backend

## Background
Solgraph seeks to make data about a number of Solana NFT projects easily digestible for NFT buyers.
The data provided will show you hourly/daily/weekly trends about a particular collection.

This repository houses the code responsible for fetching the data and inserting data into our PostgreSQL
database that uses the TimescaleDB extension for time-series data.

## Stack
* Rust --> For awesomeness, fast, robust, simple, and safe code is the best code, that's why we use Rust.
* Tokio --> For asynchronous calls.
* Surf --> For HTTP requests.
* Serde JSON --> For parsing JSON data.
* PostgreSQL --> The best database.
* TimescaleDB --> Better than InfluxDB.

## Code Style
* For Rust we use [fmt-rfcs](https://github.com/rust-dev-tools/fmt-rfcs)
* For SQL we use [SQL Style Guide by Simon Holywell](https://www.sqlstyle.guide/)