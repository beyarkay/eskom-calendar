DROP TABLE IF EXISTS geofences CASCADE;

DROP TABLE IF EXISTS aliases CASCADE;

DROP TABLE IF EXISTS municipalities CASCADE;

DROP TABLE IF EXISTS schedules CASCADE;

DROP TABLE IF EXISTS urls CASCADE;

DROP TABLE IF EXISTS places CASCADE;

-- Many places have information sources which need to be referenced, but these
-- are often long URLs that get duplicated. They all get stored as a URL ID
-- which is referenced here.
CREATE TABLE
  urls (
    id VARCHAR(7) PRIMARY KEY,
    url TEXT NOT NULL UNIQUE
  );

-- All the schedules for all the places. This references a CSV file that's on
-- disk, and provides a means to verify where the informaiton came from via
-- sources_id and info_id.
CREATE TABLE
  schedules (
    id VARCHAR(7) PRIMARY KEY,
    filename VARCHAR(255) NOT NULL,
    sources_id VARCHAR(7) REFERENCES urls (id),
    info_id VARCHAR(7) REFERENCES urls (id),
    last_updated TIMESTAMP,
    valid_from TIMESTAMP,
    valid_until TIMESTAMP
  );

-- Every municipality in South Africa, as defined by Wikipedia
CREATE TABLE
  municipalities (
    id VARCHAR(7) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    province VARCHAR(255) CHECK (
      province IN (
        'eastern_cape',
        'free_state',
        'gauteng',
        'kwazulu-natal',
        'limpopo',
        'mpumalanga',
        'north_west',
        'northern_cape',
        'western_cape'
      )
    ),
    kind VARCHAR(20) CHECK (kind IN ('local', 'district', 'metropolitan'))
  );

-- It's pretty common for places to have multiple names or different names in
-- different languages. Define a table to contain all these aliases, so that a
-- place might have one canonical name (which is what will be displayed) but
-- multiple aliases
CREATE TABLE
  aliases (
    id VARCHAR(7) PRIMARY KEY,
    alias VARCHAR(255) NOT NULL
  );

-- Each "place" in South Africa. This is a bit tricky to get an absolute
-- definition for, but this is the thing a user will look up if they want to
-- know their loadshedding schedule. It's associated with a number of aliases,
-- a geofence, and a municipality
CREATE TABLE
  places (
    id VARCHAR(7) PRIMARY KEY,
    schedule_id VARCHAR(7) REFERENCES schedules (id),
    display_name TEXT NOT NULL,
    munic_id VARCHAR(7) REFERENCES municipalities (id),
    name_aliases_id VARCHAR(7) REFERENCES aliases (id)
  );

-- Store geofences for different places. A geofence has one "id", and is made
-- up of multiple discontinuous "paths", and each path contains multiple
-- "points".
CREATE TABLE
  geofences (
    id VARCHAR(7),
    lat NUMERIC(10, 6) NOT NULL,
    lng NUMERIC(10, 6) NOT NULL,
    path_index INTEGER NOT NULL,
    point_index INTEGER NOT NULL,
    place_id VARCHAR(7) REFERENCES places (id)
  );
