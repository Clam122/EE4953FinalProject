#!/usr/bin/env python3
import requests
import sys

BASE_URL = None

def prompt(msg):
    try:
        return input(msg)
    except (KeyboardInterrupt, EOFError):
        print("\nbye")
        sys.exit(0)

def req(method, path, **kwargs):
    try:
        r = requests.request(method, BASE_URL + path, **kwargs)
        return r
    except requests.exceptions.ConnectionError:
        print(f"connection failed: {BASE_URL}")
        return None

def print_response(r):
    if r is None:
        return
    try:
        data = r.json()
        if isinstance(data, list):
            for item in data:
                print(item)
        else:
            for k, v in data.items():
                print(f"{k}: {v}")
    except Exception:
        print(r.text)

# --- actions ---

def list_users():
    r = req("GET", "/users")
    print_response(r)

def get_user():
    uid = prompt("user id: ")
    r = req("GET", f"/users/{uid}")
    print_response(r)

def get_user_by_email():
    email = prompt("email: ")
    r = req("GET", f"/getuserbyemail/{email}")
    print_response(r)

def create_user():
    email = prompt("email: ")
    public_key = prompt("public key: ")
    hash_verify = prompt("password/hash: ")
    r = req("POST", "/users", json={
        "email": email,
        "public_key": public_key,
        "hash_verify": hash_verify
    })
    print_response(r)

def delete_user():
    uid = prompt("user id: ")
    hash_verify = prompt("password/hash: ")
    r = req("DELETE", f"/users/{uid}", json={"hash_verify": hash_verify})
    print_response(r)

def update_user():
    uid = prompt("user id: ")
    hash_verify = prompt("password/hash: ")

    payload = {"hash_verify": hash_verify}

    new_email = prompt("new email (leave blank to skip): ").strip()
    if new_email:
        payload["email"] = new_email

    new_key = prompt("new public key (leave blank to skip): ").strip()
    if new_key:
        payload["public_key"] = new_key

    new_hash = prompt("new password/hash (leave blank to skip): ").strip()
    if new_hash:
        payload["new_hash_verify"] = new_hash

    vis = prompt("change visibility? (y/n/blank): ").strip().lower()
    if vis == "y":
        payload["visible_to_public"] = True
    elif vis == "n":
        payload["visible_to_public"] = False

    r = req("PUT", f"/users/{uid}", json=payload)
    print_response(r)

def pull_mirror():
    url = prompt("remote server (host:port): ")
    r = req("GET", f"/get_mirror/{url}")
    print_response(r)

def show_mirror():
    r = req("GET", "/mirror")
    print_response(r)

ACTIONS = {
    "1": ("list users",         list_users),
    "2": ("get user by id",     get_user),
    "3": ("get user by email",  get_user_by_email),
    "4": ("create user",        create_user),
    "5": ("update user",        update_user),
    "6": ("delete user",        delete_user),
    "7": ("show mirror data",   show_mirror),
    "8": ("pull from mirror",   pull_mirror),
    "q": ("quit",               lambda: sys.exit(0)),
}

def menu():
    print()
    for key, (label, _) in ACTIONS.items():
        print(f"  {key}. {label}")
    print()

def main():
    global BASE_URL

    host = prompt("server (host:port): ").strip().rstrip("/")
    if not host.startswith("http"):
        host = "http://" + host

    BASE_URL = host

    r = req("GET", "/index")
    if r is None:
        print("could not reach server")
        sys.exit(1)
    print(r.text)

    while True:
        menu()
        choice = prompt("> ").strip().lower()
        if choice in ACTIONS:
            ACTIONS[choice][1]()
        else:
            print("unknown option")

if __name__ == "__main__":
    main()