create table credentials
(
    club_id integer not null,
    app_key text not null,
    username text not null,
    password text not null
);

create table articles
(
    id text not null
        constraint articles_pk
            primary key,
    designation text not null,
    barcode text not null,
    prices blob not null
);

create table members
(
    id text not null
        constraint members_pk
            primary key,
    firstname text not null,
    lastname text not null,
    nickname text not null,
    keycodes blob not null
);

create table sales
(
    id text not null
        constraint sales_pk
            primary key,
    date text not null,
    member_id text not null,
    article_id text not null,
    amount integer not null
);
