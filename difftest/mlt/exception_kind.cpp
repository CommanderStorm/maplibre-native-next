// The kind of a C++ exception for the //difftest/mlt dumps (as in
// //difftest/tile): built with RTTI to read the dynamic type name, because
// catching standard exceptions by type in the -fno-rtti reference would
// reference typeinfo whose weak copies drag the renderer into the link.

#include <exception>
#include <string>
#include <typeinfo>

std::string exceptionKind(std::exception_ptr e, std::string& what) {
    try {
        std::rethrow_exception(e);
    } catch (const std::exception& ex) {
        what = ex.what();
        const std::string type = typeid(ex).name();
        if (type == "St13runtime_error") {
            return "runtime";
        }
        if (type == "St12out_of_range") {
            return "out_of_range";
        }
        return "other(" + type + ")";
    } catch (...) {
        what = "";
        return "unknown";
    }
}
