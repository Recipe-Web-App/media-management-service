# ADR-001: Choice of Axum Web Framework

## Status

Accepted

## Context

We need to choose a web framework for building a production-ready media management HTTP API in Rust. The service needs
to handle file uploads, downloads, and processing with high performance and type safety.

## Decision

We will use Axum 0.8 as our web framework.

## Rationale

### Advantages of Axum

- **Type Safety**: Compile-time verification of request handlers and extractors
- **Performance**: Built on Hyper and Tower, excellent async performance
- **Ecosystem**: Strong integration with Tower middleware ecosystem
- **Ergonomics**: Clean, intuitive API with excellent error handling
- **Maintainance**: Actively maintained by the Tokio team
- **Production Ready**: Used in production by many companies

### Alternatives Considered

#### Actix-Web

- **Pros**: Very fast, mature ecosystem, well-documented
- **Cons**: Less type-safe extractors, more complex actor model

#### Warp

- **Pros**: Functional approach, good performance
- **Cons**: Complex filter composition, steeper learning curve

#### Rocket

- **Pros**: Very ergonomic, excellent docs
- **Cons**: Slower compile times, less mature async support

## Consequences

### Positive

- Type-safe request handling reduces runtime errors
- Excellent middleware ecosystem for production features
- Strong async foundation for handling concurrent uploads
- Easy integration with SQLx and other async libraries

### Negative

- Smaller community compared to Actix-Web
- Fewer examples and tutorials available
- Compile times may be longer due to heavy type inference

## Implementation Notes

- Use Axum's extractors for request validation
- Leverage Tower middleware for cross-cutting concerns
- Structure handlers to work well with our Clean Architecture approach
