// The C++ oracle of //difftest/layers as a process: layers_difftest.rs
// writes one JSON request per line and reads the report (reference.cpp)
// followed by a line "<<<END>>>". A process because the Linux platform
// (mbgl-default + platform/linux, which has the LayerManager with the render
// layer factories) only links completely into C++ executables.

#include <iostream>
#include <string>

std::string layersReport(const std::string& json);

int main() {
    std::ios::sync_with_stdio(false);
    std::string line;
    while (std::getline(std::cin, line)) {
        std::cout << layersReport(line) << "<<<END>>>\n" << std::flush;
    }
    return 0;
}
