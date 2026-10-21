CREATE TABLE IF NOT EXISTS game_submission_setting (
    game_id    BIGINT      PRIMARY KEY,
    formats    TEXT[]      NOT NULL DEFAULT ARRAY['pdf'],
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT ck_game_submission_setting_not_empty CHECK (cardinality(formats) > 0)
);
