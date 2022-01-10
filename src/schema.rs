table! {
    demo_auction_info (id) {
        id -> Int4,
        last_tend -> Int4,
        item -> Text,
        end_time -> Timestamp,
        start_price -> Int4,
        bin_price -> Nullable<Int4>,
    }
}

table! {
    channel_auction (channel) {
        channel -> Int8,
        auction -> Nullable<Int4>,
    }
}
