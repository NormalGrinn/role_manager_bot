CREATE TABLE categories (
    role_id INTEGER PRIMARY KEY,
    category_name TEXT UNIQUE,
    is_open INTEGER NOT NULL,
    category_limit INTEGER NOT NULL,
    category_type TEXT NOT NULL,
    host_role INTEGER NOT NULL 
);