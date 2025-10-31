-- Delete Stake Registrations before slot number.
DELETE FROM stake_registration
WHERE slot_no <= ?;
