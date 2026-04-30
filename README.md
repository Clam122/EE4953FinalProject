# keyserver

A proof-of-concept distributed keyserver built with Rust (Axum + Diesel) and SQLite. Users can register an email and RSA public key. Servers can mirror each other's public user data over HTTP.

## Dependencies

- Rust (https://rustup.rs)
- Python 3
- `libsqlite3-dev` (Linux only)

## Setup

```bash
# Linux only
sudo apt install libsqlite3-dev

# Install the Diesel CLI
cargo install diesel_cli --no-default-features --features sqlite

# Clone the repo
git clone <repo>
cd keyserver

# Create the environment file
echo DATABASE_URL=db.sqlite3 > .env

# Initialize the database and run migrations
diesel setup
diesel migration run

# Start the server
cargo run
```

The server listens on `http://localhost:5000`.

## Client

```bash
pip install requests
python client.py
```

On startup the client prompts for a server address, confirms the connection, then presents a menu of available actions.

## API

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/index` | Server info |
| GET | `/users` | List all public users |
| GET | `/users/:id` | Get user by ID |
| GET | `/getuserbyemail/:email` | Get user by email |
| POST | `/users` | Create a user |
| PUT | `/users/:id` | Update a user |
| DELETE | `/users/:id` | Delete a user |
| GET | `/mirror` | Export public users for mirroring |
| GET | `/get_mirror/:host` | Pull and merge users from a remote server |

All write operations require the user's `hash_verify` password in the request body for authentication.

## Mirroring

Servers can replicate public user data from each other. A `GET /get_mirror/<host:port>` request will pull all public users from the remote server's `/mirror` endpoint and merge them into the local database, skipping any entries with duplicate emails or public keys. Mirrored users are marked with a placeholder hash and cannot be modified locally.

## Notes

- `visible_to_public` defaults to `false` on registration. Users must explicitly opt in to appear in public listings and mirrors.
- The database file (`db.sqlite3`) and `.env` are gitignored and must be created locally.