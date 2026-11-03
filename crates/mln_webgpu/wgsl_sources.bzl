"""Runs the WGSL extractor over the C++ shader headers.

A plain Starlark rule instead of a genrule: genrules need a shell
(/bin/bash), which NixOS does not have at that path.
"""

def _wgsl_sources_impl(ctx):
    out = ctx.outputs.out
    ctx.actions.run(
        executable = ctx.executable.tool,
        arguments = [out.path] + [f.path for f in ctx.files.srcs],
        inputs = ctx.files.srcs,
        outputs = [out],
        mnemonic = "ExtractWgsl",
        progress_message = "Extracting WGSL into %s" % out.short_path,
    )
    return [DefaultInfo(files = depset([out]))]

wgsl_sources = rule(
    implementation = _wgsl_sources_impl,
    attrs = {
        "srcs": attr.label_list(allow_files = [".hpp"], mandatory = True),
        "out": attr.output(mandatory = True),
        "tool": attr.label(executable = True, cfg = "exec", mandatory = True),
    },
)
