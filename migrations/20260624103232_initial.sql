-- Add migration script here
CREATE TABLE "battery_types" (
	"id"	INTEGER PRIMARY KEY,
	"manufacturer"	TEXT NOT NULL,
	"model"	TEXT NOT NULL,
	"chemistry"	TEXT NOT NULL,
	"nominal_voltage_mv"	INTEGER NOT NULL,
	"nominal_capacity_mah"	INTEGER NOT NULL,
	"charge_termination_voltage_mv"	INTEGER NOT NULL,
	"discharge_cutoff_voltage_mv"	INTEGER NOT NULL,
	"notes"	TEXT,
	UNIQUE("manufacturer","model")
);

CREATE TABLE batteries (
    battery_id TEXT PRIMARY KEY,

    battery_type_id INTEGER NOT NULL,

    notes TEXT,

    FOREIGN KEY (battery_type_id)
        REFERENCES battery_types(id)
);

CREATE TABLE battery_intake (
    battery_id TEXT PRIMARY KEY,

    serial_number TEXT,

    purchase_date TEXT,
    delivery_date TEXT,

    voltage_at_delivery_mv INTEGER,
    internal_resistance_at_delivery_uohm INTEGER,
	visual_inspection TEXT,

    notes TEXT,

    FOREIGN KEY (battery_id)
        REFERENCES batteries(battery_id)
);

CREATE TABLE "test_modes" (
	"acronym"	TEXT PRIMARY KEY,
	"description"	TEXT NOT NULL,
	UNIQUE("description")
);

CREATE TABLE tests (
    id INTEGER PRIMARY KEY,
    battery_id TEXT NOT NULL,
    approved INTEGER NOT NULL DEFAULT 0,
    device_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    voltage_before_test_mv INTEGER NOT NULL,

    measured_capacity_mah INTEGER,
    measured_energy_mwh INTEGER,
    end_voltage_mv INTEGER,
    notes TEXT,

    FOREIGN KEY(battery_id) REFERENCES batteries(battery_id),
    FOREIGN KEY(mode) REFERENCES test_modes(acronym)
);

CREATE UNIQUE INDEX idx_tests_one_approved_per_battery
ON tests(battery_id)
WHERE approved = 1;

CREATE TABLE "test_sessions" (
    "id" INTEGER PRIMARY KEY,
    "test_id" INTEGER NOT NULL,
    "started_at" TEXT NOT NULL,
    "ended_at" TEXT,
    "reason" TEXT,
    UNIQUE("test_id", "started_at"),
    FOREIGN KEY("test_id") REFERENCES "tests"("id")
);

CREATE TABLE "samples" (
    "session_id" INTEGER NOT NULL,
    "sample_index" INTEGER NOT NULL,
    "timestamp" TEXT NOT NULL,
    "elapsed_ms" INTEGER NOT NULL,
    "voltage_mv" INTEGER NOT NULL,
    "current_ma" INTEGER NOT NULL,
    "capacity_mah" INTEGER NOT NULL,
    "energy_mwh" INTEGER,
    PRIMARY KEY("session_id","sample_index"),
    FOREIGN KEY("session_id") REFERENCES "test_sessions"("id")
);

CREATE TABLE discharge_cc_tests (
    test_id INTEGER PRIMARY KEY,
    target_current_ma INTEGER NOT NULL,
    cutoff_voltage_mv INTEGER NOT NULL,
    cutoff_time_min INTEGER NOT NULL,

    FOREIGN KEY(test_id) REFERENCES tests(id) ON DELETE CASCADE
);

CREATE TABLE discharge_cp_tests (
    test_id INTEGER PRIMARY KEY,
    target_power_w INTEGER NOT NULL,
    cutoff_voltage_mv INTEGER NOT NULL,
    cutoff_time_min INTEGER NOT NULL,

    FOREIGN KEY(test_id) REFERENCES tests(id) ON DELETE CASCADE
);

CREATE TABLE charge_cv_tests (
    test_id INTEGER PRIMARY KEY,
    target_current_ma INTEGER NOT NULL,
    charge_voltage_mv INTEGER NOT NULL,
    charge_cutoff_current_ma INTEGER NOT NULL,

    FOREIGN KEY(test_id) REFERENCES tests(id) ON DELETE CASCADE
);

INSERT INTO test_modes (acronym, description) VALUES
    ('DSC-CC', 'Discharge constant current'),
    ('DSC-CP', 'Discharge constant power'),
    ('CHG-CV', 'Charge constant voltage');