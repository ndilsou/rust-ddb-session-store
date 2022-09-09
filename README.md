# Rust Serverless Session Store

This is a demo app I build to play with Rust as a language for serverless app. 
The application itself is a fairly basic key value store backed by DynamoDB and using either API Gateway HTTP API or Application Load Balancer.

As expected it is.... blazingly fast. All the request latency comes from the network with API Gateway adding the most.

## Build Dependencies

* cargo-lambda
