CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    owner_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_projects_owner ON projects(owner_id);

CREATE TABLE novels (
    id               TEXT PRIMARY KEY,
    project_id       TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title            TEXT NOT NULL,
    synopsis         TEXT NOT NULL DEFAULT '',
    genre            TEXT NOT NULL DEFAULT '',
    style            TEXT NOT NULL DEFAULT '',
    pov              TEXT NOT NULL DEFAULT 'third',
    tone             TEXT NOT NULL DEFAULT '',
    target_word_count INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);
CREATE INDEX idx_novels_project ON novels(project_id);

CREATE TABLE chapters (
    id           TEXT PRIMARY KEY,
    novel_id     TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    summary      TEXT NOT NULL DEFAULT '',
    content      TEXT NOT NULL DEFAULT '',
    order_index  INTEGER NOT NULL,
    status       TEXT NOT NULL DEFAULT 'draft',
    word_count   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    UNIQUE(novel_id, order_index)
);
CREATE INDEX idx_chapters_novel ON chapters(novel_id);

CREATE TABLE characters (
    id          TEXT PRIMARY KEY,
    novel_id    TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'supporting',
    description TEXT NOT NULL DEFAULT '',
    traits      TEXT NOT NULL DEFAULT '',
    backstory   TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_characters_novel ON characters(novel_id);

CREATE TABLE outline_nodes (
    id          TEXT PRIMARY KEY,
    novel_id    TEXT NOT NULL REFERENCES novels(id) ON DELETE CASCADE,
    parent_id   TEXT REFERENCES outline_nodes(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    summary     TEXT NOT NULL DEFAULT '',
    order_index INTEGER NOT NULL,
    chapter_id  TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_outline_novel ON outline_nodes(novel_id);
CREATE INDEX idx_outline_parent ON outline_nodes(parent_id);

CREATE TABLE chapter_characters (
    chapter_id   TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    PRIMARY KEY (chapter_id, character_id)
);
