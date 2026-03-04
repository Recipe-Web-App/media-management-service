# Media Management Service - Database Schema Reference

Schema managed externally. This service is a consumer, not the owner. No migrations in this repo.

The service connects to PostgreSQL database `recipe_database` and queries tables in the `recipe_manager` schema.

## Tables

### `recipe_manager.media`

Core media metadata. One row per uploaded file.

| Column            | Type        | Nullable | Description                                                                                      |
| ----------------- | ----------- | -------- | ------------------------------------------------------------------------------------------------ |
| media_id          | BIGSERIAL   | NO       | Primary key, auto-increment                                                                      |
| user_id           | UUID        | NO       | Owning user's ID (maps to `uploaded_by` in API responses)                                        |
| content_hash      | VARCHAR(64) | NO       | SHA-256 hex hash (unique)                                                                        |
| original_filename | VARCHAR     | YES      | Original uploaded filename                                                                       |
| media_type        | VARCHAR     | NO       | MIME type (e.g., "image/jpeg")                                                                   |
| media_path        | VARCHAR     | NO       | CAS storage path (e.g., "ab/cd/ef/abcdef..."). **Internal only — not exposed in API responses.** |
| file_size         | BIGINT      | NO       | File size in bytes                                                                               |
| processing_status | VARCHAR     | NO       | pending, processing, complete, failed (lowercase)                                                |
| created_at        | TIMESTAMPTZ | NO       | Upload timestamp                                                                                 |
| updated_at        | TIMESTAMPTZ | NO       | Last status update                                                                               |

**Indexes:**

- PRIMARY KEY on `media_id`
- UNIQUE INDEX on `content_hash`
- INDEX on `user_id`
- INDEX on `created_at`

### `recipe_manager.recipe_media`

Links media to recipes.

| Column    | Type   | Nullable | Description   |
| --------- | ------ | -------- | ------------- |
| recipe_id | BIGINT | NO       | FK to recipes |
| media_id  | BIGINT | NO       | FK to media   |

### `recipe_manager.ingredient_media`

Links media to recipe ingredients.

| Column        | Type   | Nullable | Description       |
| ------------- | ------ | -------- | ----------------- |
| recipe_id     | BIGINT | NO       | FK to recipes     |
| ingredient_id | BIGINT | NO       | FK to ingredients |
| media_id      | BIGINT | NO       | FK to media       |

### `recipe_manager.step_media`

Links media to recipe steps.

| Column    | Type   | Nullable | Description   |
| --------- | ------ | -------- | ------------- |
| recipe_id | BIGINT | NO       | FK to recipes |
| step_id   | BIGINT | NO       | FK to steps   |
| media_id  | BIGINT | NO       | FK to media   |

## SQL Query Reference

These queries are carried forward from the current implementation. They are correct and well-tested.

### Insert Media

```sql
INSERT INTO recipe_manager.media
    (user_id, media_type, media_path, file_size, content_hash,
     original_filename, processing_status, created_at, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING media_id
```

### Find by ID

```sql
SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
       original_filename, processing_status, created_at, updated_at
FROM recipe_manager.media
WHERE media_id = $1
```

### Find by Content Hash

```sql
SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
       original_filename, processing_status, created_at, updated_at
FROM recipe_manager.media
WHERE content_hash = $1
```

### Find by User (All)

```sql
SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
       original_filename, processing_status, created_at, updated_at
FROM recipe_manager.media
WHERE user_id = $1
ORDER BY created_at DESC
```

### Paginated Query (Cursor-Based)

Dynamically built with optional filters:

```sql
SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
       original_filename, processing_status, created_at, updated_at
FROM recipe_manager.media
WHERE user_id = $1
  [AND processing_status = $N]   -- optional status filter
  [AND media_id > $N]            -- optional cursor (for next page)
ORDER BY media_id ASC
LIMIT $N                         -- limit + 1 to detect has_next
```

**Cursor encoding**: Base64-encoded `media_id` of the last item in the current page. Decode on next request to use as `WHERE media_id > cursor`. Fetch `limit + 1` rows; if more than `limit` returned, there is a next page.

### Update Media

```sql
UPDATE recipe_manager.media
SET media_type = $2, media_path = $3, file_size = $4, content_hash = $5,
    original_filename = $6, processing_status = $7, updated_at = $8
WHERE media_id = $1
```

### Delete Media

```sql
DELETE FROM recipe_manager.media
WHERE media_id = $1
```

### Check Existence by Hash

```sql
SELECT EXISTS(
    SELECT 1 FROM recipe_manager.media WHERE content_hash = $1
) AS exists
```

### Recipe Association Queries

```sql
-- Media IDs by recipe
SELECT media_id
FROM recipe_manager.recipe_media
WHERE recipe_id = $1
ORDER BY media_id

-- Media IDs by ingredient
SELECT media_id
FROM recipe_manager.ingredient_media
WHERE recipe_id = $1 AND ingredient_id = $2
ORDER BY media_id

-- Media IDs by step
SELECT media_id
FROM recipe_manager.step_media
WHERE recipe_id = $1 AND step_id = $2
ORDER BY media_id
```

### Health Check

```sql
SELECT 1
```

## Rust Model Mapping

### Write Model (insert)

```rust
pub struct NewMedia {
    pub user_id: Uuid,
    pub content_hash: String,        // Validated 64-char hex
    pub original_filename: String,
    pub media_type: String,          // MIME type
    pub media_path: String,          // CAS path
    pub file_size: i64,
    pub processing_status: String,   // pending | processing | complete | failed
}
```

### Read Model (query result)

```rust
pub struct Media {
    pub media_id: i64,
    pub user_id: Uuid,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String,
    pub media_path: String,
    pub file_size: i64,
    pub processing_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### API Response DTO

The `MediaDto` returned in API responses omits `media_path` (internal) and renames `user_id` to `uploaded_by`. It also includes a computed `download_url` field (signed URL, null if not complete).

```rust
#[derive(Serialize)]
pub struct MediaDto {
    pub id: i64,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String,
    pub file_size: i64,
    pub processing_status: String,
    pub uploaded_by: String,         // from user_id
    pub uploaded_at: DateTime<Utc>,  // from created_at
    pub updated_at: DateTime<Utc>,
    pub download_url: Option<String>, // computed: signed URL if status == complete
}
```

### Processing Status Lifecycle

```
pending ──> processing ──> complete
                │
                └──────────> failed
```

- **pending**: Media record created (presigned upload initiated), file not yet received
- **processing**: File received and being hashed/stored
- **complete**: File stored successfully, ready for download
- **failed**: Storage or processing error

For direct uploads (POST /media/), the status goes directly to COMPLETE since hashing and storage happen synchronously.

For presigned uploads, the flow is PENDING -> COMPLETE (or FAILED) when the client uploads to the signed URL.

## Notes

- `content_hash` is the SHA-256 hex digest of the file contents (always 64 characters, lowercase)
- `media_path` stores the CAS-relative path (e.g., `ab/cd/ef/abcdef1234...`), not the full filesystem path
- `file_size` is stored as BIGINT (i64 in Rust) to support files up to ~9.2 EB
- `user_id` is a UUID matching the auth-service user identity
- The `ContentHash` value object in `models.rs` should validate: exactly 64 chars, all hex digits, lowercase
