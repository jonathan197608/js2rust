// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 180
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_180.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_180() {

        const fileName = "my file(2).txt";
        const header = `Content-Disposition: attachment; filename*=UTF-8''${encodeRFC5987ValueChars(
          fileName,
        )}`;

        console.log(header);
        // "Content-Disposition: attachment; filename*=UTF-8''my%20file%282%29.txt"

        function encodeRFC5987ValueChars(str) {
          return (
            encodeURIComponent(str)
              // The following creates the sequences %27 %28 %29 %2A (Note that
              // the valid encoding of "*" is %2A, which necessitates calling
              // toUpperCase() to properly encode). Although RFC3986 reserves "!",
              // RFC5987 does not, so we do not need to escape it.
              .replace(
                /['()*]/g,
                (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
              )
              // The following are not required for percent-encoding per RFC5987,
              // so we can allow for a little better readability over the wire: |`^
              .replace(/%(7C|60|5E)/g, (str, hex) =>
                String.fromCharCode(parseInt(hex, 16)),
              )
          );
        }
    }
