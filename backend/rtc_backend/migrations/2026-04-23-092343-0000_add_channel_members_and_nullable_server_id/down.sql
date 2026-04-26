DROP TABLE channel_members;

ALTER TABLE channels ALTER COLUMN server_id SET NOT NULL;