-- Database drop/creation

-- DROP DATABASE IF NOT EXISTS tf_backend;
CREATE DATABASE IF NOT EXISTS tf_backend;

-- Table drop/creation

-- DROP TABLE IF NOT EXISTS tf_backend.tfstate;
CREATE TABLE IF NOT EXISTS tf_backend.tfstates ( 
    id INT AUTO_INCREMENT NOT NULL COMMENT 'Unique identifier' ,
    name VARCHAR(50) NOT NULL DEFAULT 'default'  COMMENT 'Unique name of the state' ,
    contents JSON NOT NULL COMMENT 'JSON contents of the state file',
    created_at TIMESTAMP NOT NULL COMMENT 'Creation time' ,
    last_changed TIMESTAMP NOT NULL COMMENT 'Last time the state changed' ,
    CONSTRAINT states_PK PRIMARY KEY (id),
    CONSTRAINT states_name_UK UNIQUE (name)
);

-- DROP TABLE IF NOT EXISTS tf_backend.locked_tfstates;
CREATE TABLE IF NOT EXISTS tf_backend.locked_tfstates (
    name VARCHAR(50) NOT NULL,
    CONSTRAINT locked_PK PRIMARY KEY(name)
);
