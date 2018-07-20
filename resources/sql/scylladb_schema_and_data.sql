CREATE KEYSPACE staging
  WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 3};

USE staging;

CREATE TABLE gw_known_ifas (
  app_id uuid,
  ifa uuid,
  entity_id uuid,
  PRIMARY KEY (app_id, ifa)
) WITH comment='xorc gateway IFA matching';

CREATE TABLE gw_application_access (
  app_id uuid,
  sdk_token text,
  ios_secret text,
  android_secret text,
  web_secret text,
  PRIMARY KEY (app_id)
) WITH comment='xorc gateway application access tokens';

INSERT INTO gw_application_access (
  app_id,
  sdk_token,
  ios_secret,
  android_secret,
  web_secret
) VALUES (
  a2faae91-d52f-497d-9029-d91be08c28c5,
  '46732a28cd445366c6c8dcbd57500af4e69597c8ebe224634d6ccab812275c9c',
  '1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5',
  '1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5',
  '4c553960fdc2a82f90b84f6ef188e836818fcee2c43a6c32bd6c91f41772657f'
);
