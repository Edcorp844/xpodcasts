-- 1. Rename the old table
ALTER TABLE shows RENAME TO old_table;

-- 2. Create the new table with all old and new columns
CREATE TABLE shows
(
    `id`                     INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    `title`                  TEXT    NOT NULL,
    `link`                   TEXT    NOT NULL,
    `description`            TEXT    NOT NULL,
    `image_uri`              TEXT,
    `image_uri_hash`         BLOB,    -- Missing in your version!
    `image_cached`           TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    `source_id`              INTEGER NOT NULL, -- Removed UNIQUE
    
    -- New extended metadata
    `censored_name`          TEXT,
    `primary_genre`          TEXT,
    `genres`                 TEXT,
    `genre_ids`              TEXT,
    `is_explicit`            BOOLEAN,
    `content_advisory_rating` TEXT,
    `apple_url`              TEXT,
    `artwork_url_30`         TEXT,
    `artwork_url_60`         TEXT,
    `artwork_url_100`        TEXT,
    `track_time_millis`      BIGINT
);

-- 3. Copy the old data over. 
-- New columns will automatically default to NULL.
INSERT INTO shows (
    id, title, link, description, image_uri, image_uri_hash, 
    image_cached, source_id
)
SELECT 
    id, title, link, description, image_uri, image_uri_hash, 
    image_cached, source_id
FROM old_table;

-- 4. Drop the old table
DROP TABLE old_table;