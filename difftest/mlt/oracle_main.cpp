// The C++ oracle of //difftest/mlt as an executable: the C++ MLT decoder
// can fail an `assert` or read out of bounds on malformed tiles, which must
// not take the Rust test down with it. mlt_difftest.rs drives it over
// stdin/stdout. Request: u8 opcode + u32 length + payload; answer: u32
// length + payload.
//
// Opcodes: 0 = NDEBUG? (answer "1"/"0"); 1/2 = decode the payload with
// fastPFOREnabled false/true (answer: reference.cpp's dump).

#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

std::string mltDump(const uint8_t* data, size_t size, bool fastPFOR);
bool mltNdebug();

namespace {

bool readExact(void* buf, size_t n) {
    return std::fread(buf, 1, n, stdin) == n;
}

void answer(const std::string& s) {
    const uint32_t n = static_cast<uint32_t>(s.size());
    std::fwrite(&n, 4, 1, stdout);
    std::fwrite(s.data(), 1, s.size(), stdout);
    std::fflush(stdout);
}

} // namespace

int main() {
    while (true) {
        uint8_t op;
        uint32_t len;
        if (!readExact(&op, 1) || !readExact(&len, 4)) return 0;
        std::vector<uint8_t> payload(len);
        if (len && !readExact(payload.data(), len)) return 1;
        switch (op) {
            case 0:
                answer(mltNdebug() ? "1" : "0");
                break;
            case 1:
            case 2:
                answer(mltDump(payload.data(), payload.size(), op == 2));
                break;
            default:
                return 2;
        }
    }
}
