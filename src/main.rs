mod state;

use state::machine;

fn main() {
	test = state::machine::StateMachine;
	let teststr = r#"
		{sequence_code":"1", "sequence": [
		{"valve":"GOVNT1", "state":"open", "time_ms":100},
		{"valve":"GOVNT2", "state":"open", "time_ms":1100},
		{"valve":"GOFILL", "state":"open", "time_ms":2100},
		{"valve":"GPRFILL", "state":"open", "time_ms":3100},
		{"valve":"GPRVNT", "state":"open", "time_ms":4100},
		{"valve":"OOQD", "state":"open", "time_ms":5100},
		{"valve":"PRQD", "state":"open", "time_ms":6100},
		{"valve":"GOVNT1", "state":"close", "time_ms":7100},
		{"valve":"GOVNT2", "state":"close", "time_ms":8100},
		{"valve":"GOFILL", "state":"close", "time_ms":9100},
		{"valve":"GPRFILL", "state":"close", "time_ms":10100},
		{"valve":"GPRVNT", "state":"close", "time_ms":11100},
		{"valve":"OOQD", "state":"close", "time_ms":12100},
		{"valve":"PRQD", "state":"close", "time_ms":13100, }
		]
	}"#;
	test.runSequence(teststr);
}
