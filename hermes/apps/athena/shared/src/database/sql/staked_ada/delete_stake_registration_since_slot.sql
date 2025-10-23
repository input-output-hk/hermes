-- Delete Stake Registrations since slot number.
DELETE FROM stake_registration
WHERE slot_no >= ?;
