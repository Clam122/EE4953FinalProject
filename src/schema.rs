// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Integer,
        email -> Text,
        public_key -> Text,
        hash_verify -> Text,
        visible_to_public -> Integer,
    }
}
