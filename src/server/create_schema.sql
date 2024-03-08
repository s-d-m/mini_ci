PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000; -- Release lock after 5 seconds

BEGIN;

CREATE TABLE IF NOT EXISTS job_status (
    id INTEGER PRIMARY KEY NOT NULL,
    human_name TEXT UNIQUE NOT NULL
);

INSERT OR IGNORE INTO job_status(id, human_name)
VALUES
    (1, 'pending'),
    (2, 'running'),
    (3, 'success'),
    (4, 'failed'),
    (5, 'timeout'),
    (6, 'skipped');

CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    commit_id VARCHAR(100) NOT NULL,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    status INTEGER DEFAULT 1,
    email VARCHAR(100) DEFAULT NULL, -- unused. Was in prevision of sending notifications on completion
    FOREIGN KEY(status) REFERENCES job_status(id) ON DELETE CASCADE,
    CHECK ( ((email LIKE '%@%') AND (length(email) >= 3)) or (email is null ))
);

CREATE INDEX IF NOT EXISTS jobs_to_status ON jobs(status);

CREATE TABLE IF NOT EXISTS tasks_kind(
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT UNIQUE NOT NULL
);

INSERT OR IGNORE INTO tasks_kind(id, name)
  VALUES
    (1, 'static_analyser'),
    (2, 'clang-format'),
    (3, 'clang-tidy'),
    (4, 'tests')
  ;

CREATE TABLE IF NOT EXISTS targets(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT UNIQUE NOT NULL
);

INSERT OR IGNORE INTO targets
  VALUES
    (1, 'qemu'),
    (2, 'real_hardware')
  ;

CREATE TABLE IF NOT EXISTS compilers(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT UNIQUE NOT NULL
);

INSERT OR IGNORE INTO compilers
VALUES
  (1, 'gcc_from_hardware_vendor'),
  (2, 'gccFromDistro')
;

CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  job_id INTEGER NOT NULL, -- job this task belongs to
  status INTEGER DEFAULT 1,
  ret_code INTEGER DEFAULT NULL,
  task_type INTEGER NOT NULL,
  started_at DATETIME DEFAULT NULL,
  finished_at DATETIME DEFAULT NULL,
  executed_on TEXT DEFAULT NULL,
  output TEXT NOT NULL DEFAULT "",

  FOREIGN KEY (task_type) REFERENCES tasks_kind(id) ON DELETE CASCADE,
  FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
  FOREIGN KEY (status) REFERENCES job_status(id) ON DELETE CASCADE,
  CHECK( (ret_code is null ) OR (ret_code BETWEEN 0 AND 255) ),
  CHECK( (((status == 1) or (status == 2)) and (ret_code is null))
         or ((status != 1) and (status != 2) and (ret_code is not null) )  )
);

CREATE INDEX IF NOT EXISTS task_to_job ON tasks(job_id DESC);

CREATE TABLE IF NOT EXISTS test_type(
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL UNIQUE
);

INSERT OR IGNORE INTO test_type
VALUES
  (1, 'All tests'),
  (2, 'No tests, only compile'),
  -- 3 left out on purpose as it corresponds to "not even compile"
  (4, 'all tests except'),
  (5, 'only specified tests');

CREATE TABLE IF NOT EXISTS test_setup(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    task_id INTEGER NOT NULL,
    compiler_id INTEGER NOT NULL,
    required_tests INTEGER NOT NULL,
    mentioned_tests TEXT DEFAULT NULL,
    run_tests_on_qemu INTEGER DEFAULT 0,
    run_tests_on_real_hardware INTEGER DEFAULT 0,

    FOREIGN KEY (required_tests) REFERENCES test_type(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (compiler_id) REFERENCES compilers(id) ON DELETE CASCADE,
    CHECK( ((mentioned_tests is null) and ((required_tests == 1) or (required_tests == 2)))
           or (mentioned_tests is not null))
);

CREATE INDEX IF NOT EXISTS test_to_task ON test_setup(task_id DESC);

CREATE TABLE IF NOT EXISTS compile_output(
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  test_setup_id INTEGER NOT NULL,
  started_at DATETIME DEFAULT NULL,
  finished_at DATETIME DEFAULT NULL,
  output TEXT DEFAULT NULL,
  status INTEGER DEFAULT NULL,

  FOREIGN KEY (test_setup_id) REFERENCES test_setup(id) ON DELETE CASCADE,
  CHECK ( (finished_at is null) or (status is not null)),
  CHECK ( (finished_at is null) or (started_at is not null))
);

CREATE INDEX IF NOT EXISTS compile_output_to_test_setup ON compile_output(test_setup_id DESC);

CREATE TABLE IF NOT EXISTS test_run(
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  test_name TEXT NOT NULL,
  started_at DATETIME,
  finished_at DATETIME,
  output TEXT NOT NULL DEFAULT "",
  status INTEGER DEFAULT 1, -- pending
  ret_code INTEGER,
  target_id INTEGER NOT NULL,
  task_id INTEGER NOT NULL,

  FOREIGN KEY (status) REFERENCES job_status(id) ON DELETE CASCADE,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (target_id) REFERENCES targets(id) ON DELETE CASCADE,
  CHECK ( (ret_code is null) or (ret_code BETWEEN 0 AND 255) ),
  CHECK ((finished_at is null) or (status is not null)),
  CHECK ((finished_at is null) or (started_at is not null))
);

CREATE INDEX IF NOT EXISTS test_run_to_task ON test_run(task_id DESC, test_name ASC);

COMMIT;