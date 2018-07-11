UPDATE
  v1_entries
SET
  project_name = "default"
WHERE
  project_name IS null;
