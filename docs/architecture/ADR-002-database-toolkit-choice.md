# ADR-002: Choice of SQLx for Database Operations

## Status

Accepted

## Context

We need to choose a database toolkit for interacting with PostgreSQL to store media metadata, user information, and
processing status. The solution must be async, type-safe, and performant.

## Decision

We will use SQLx 0.8 as our database toolkit.

## Rationale

### Advantages of SQLx

- **Compile-time Verification**: SQL queries are checked at compile time against the actual database schema
- **Async Native**: Built for async/await from the ground up
- **Raw SQL**: Use actual SQL instead of a query builder DSL
- **Connection Pooling**: Built-in async connection pooling
- **Migration Support**: Built-in migration management
- **Type Safety**: Strong Rust type mapping for database types

### Alternatives Considered

#### Diesel

- **Pros**: Mature, excellent type safety, great query builder
- **Cons**: Synchronous by default, complex async integration, ORM overhead

#### SeaORM

- **Pros**: Modern async ORM, Active Record pattern
- **Cons**: Additional abstraction layer, less direct SQL control

#### Raw Database Drivers

- **Pros**: Maximum control and performance
- **Cons**: No compile-time query verification, more boilerplate

## Consequences

### Positive

- Compile-time query verification prevents SQL errors in production
- Direct SQL control for complex queries and performance optimization
- Excellent async performance with connection pooling
- Easy database migration management
- Strong type safety for data integrity

### Negative

- Requires actual database for compile-time verification
- More verbose than ORM solutions for simple CRUD operations
- Need to write SQL manually (not necessarily a negative)

## Implementation Notes

- Use SQLx macros for compile-time query verification
- Implement repository traits with SQLx as the concrete implementation
- Set up proper connection pooling configuration
- Use SQLx migrations for schema management
- Consider using query files for complex queries
