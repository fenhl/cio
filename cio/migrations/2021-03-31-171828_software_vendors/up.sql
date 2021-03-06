CREATE TABLE software_vendors (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    status VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    website VARCHAR NOT NULL,
    has_okta_integration BOOLEAN NOT NULL DEFAULT 'f',
    used_purely_for_api BOOLEAN NOT NULL DEFAULT 'f',
    pay_as_you_go BOOLEAN NOT NULL DEFAULT 'f',
    pay_as_you_go_pricing_description VARCHAR NOT NULL,
    software_licenses BOOLEAN NOT NULL DEFAULT 'f',
    cost_per_user_per_month REAL NOT NULL DEFAULT 0,
    users INTEGER DEFAULT 0 NOT NULL,
    flat_cost_per_month REAL NOT NULL DEFAULT 0,
    total_cost_per_month REAL NOT NULL DEFAULT 0,
    groups TEXT [] NOT NULL,
    airtable_record_id VARCHAR NOT NULL DEFAULT ''
)
