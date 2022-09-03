# Rust Serverless Session Store

This is a demo app I build to play with Rust as a language for serverless app. 
The application itself is a fairly basic key value store backed by DynamoDB and using API Gateway HTTP api.

As expected it is.... blazingly fast. All the request latency comes from the API Gateway.

## Build Dependencies

* cargo-lambda
