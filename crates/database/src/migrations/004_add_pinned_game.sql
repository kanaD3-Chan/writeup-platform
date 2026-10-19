CREATE TABLE IF NOT EXISTS pinned_game (
    id        SMALLINT    PRIMARY KEY,
    game_id   BIGINT      NOT NULL,
    pinned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT ck_pinned_game_single_row CHECK (id = 1)
);
