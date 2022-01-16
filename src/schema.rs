table! {
    auction_info (id) {
        id -> Int4,
        channel_id -> Int8,
        owner_id -> Int8,
        item -> Text,
        end_time -> Timestamp,
        start_price -> Int4,
        bin_price -> Nullable<Int4>,
        unit -> Text,
        embed_id -> Nullable<Int8>,
        notice -> Text,
        tenders_id -> Array<Int8>,
        tends_price -> Array<Int4>,
    }
}

table! {
    channel_auction (channel) {
        channel -> Int8,
        auction -> Nullable<Int4>,
    }
}
