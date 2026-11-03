// The compact rapidjson::Writer<StringBuffer> that value.cpp and
// conversion/stringify.hpp write JSON with: the //vendor/rapidjson port.
// The C++ callers ignore the writer's bool results; this wrapper does too.

pub(crate) struct Writer(rapidjson::Writer);

impl Writer {
    pub fn new() -> Self {
        Writer(rapidjson::Writer::new())
    }

    pub fn into_string(self) -> String {
        self.0.into_string()
    }

    pub fn null(&mut self) {
        self.0.null();
    }

    pub fn bool_(&mut self, b: bool) {
        self.0.bool_(b);
    }

    pub fn int(&mut self, i: i32) {
        self.0.int(i);
    }

    pub fn int64(&mut self, i: i64) {
        self.0.int64(i);
    }

    pub fn uint64(&mut self, u: u64) {
        self.0.uint64(u);
    }

    pub fn double(&mut self, d: f64) -> bool {
        self.0.double(d)
    }

    pub fn string(&mut self, s: &str) {
        self.0.string(s);
    }

    pub fn key(&mut self, s: &str) {
        self.0.key(s);
    }

    pub fn start_object(&mut self) {
        self.0.start_object();
    }

    pub fn end_object(&mut self) {
        self.0.end_object();
    }

    pub fn start_array(&mut self) {
        self.0.start_array();
    }

    pub fn end_array(&mut self) {
        self.0.end_array();
    }
}
