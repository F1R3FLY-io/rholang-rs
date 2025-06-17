# Using the Rholang Plugin in JetBrains IDEs

This guide will walk you through the process of installing and using the Rholang plugin in JetBrains IDEs such as IntelliJ IDEA, PyCharm, WebStorm, etc.

## Installation

### Prerequisites

- A JetBrains IDE (IntelliJ IDEA, PyCharm, WebStorm, etc.)
- The Rholang plugin zip file (`rholang-jetbrains-1.0-SNAPSHOT.zip`)

### Installation Steps

1. Open your JetBrains IDE
2. Go to **Settings/Preferences** (File > Settings on Windows/Linux, IntelliJ IDEA > Preferences on macOS)
3. Navigate to **Plugins**
4. Click the gear icon (⚙️) in the top-right corner
5. Select **Install Plugin from Disk...**
6. Browse to the location of `rholang-jetbrains-1.0-SNAPSHOT.zip` (located in the `build/distributions` directory of this project)
7. Select the zip file and click **OK**
8. Click **Restart IDE** when prompted

## Using the Plugin

Once the plugin is installed and the IDE has restarted, you can start using it to work with Rholang files.

### Opening Rholang Files

1. The plugin automatically recognizes files with the `.rho` extension as Rholang files
2. You can open the example files provided in the `examples` directory:
   - `hello.rho`: A simple "Hello World" program in Rholang
   - `calculator.rho`: A more complex example showing a calculator contract

### Features

The plugin currently provides the following features:

1. **Syntax Highlighting**: Keywords, comments, strings, numbers, and operators are highlighted with different colors
2. **File Type Recognition**: Files with the `.rho` extension are automatically recognized as Rholang files

### Example Usage

1. Open one of the example files (e.g., `examples/hello.rho`)
2. Notice the syntax highlighting that makes the code easier to read:
   - Keywords like `new`, `in`, `contract`, `for` are highlighted
   - Comments (starting with `//`) are highlighted
   - Strings (enclosed in double quotes) are highlighted
   - Numbers are highlighted
   - Operators are highlighted

## Troubleshooting

If you encounter any issues with the plugin:

1. Make sure you're using a compatible version of the JetBrains IDE (compatible with versions 2022.3 through 2025.1.*)
2. Check the IDE logs for any error messages (Help > Show Log in Explorer/Finder)
3. Try reinstalling the plugin

## Future Improvements

The current version of the plugin provides basic functionality. Future versions may include:

- Code completion
- Error highlighting
- Code formatting
- Navigation (go to definition, find usages, etc.)
- Refactoring support
- Integration with the Rholang compiler

## JNI (Java Native Interface) Integration

The Rholang plugin uses JNI to integrate with the Rholang parser written in Rust. This allows the plugin to provide advanced features like syntax validation and parsing.

### How JNI Works in the Plugin

1. The plugin includes a native library (`librholang_parser`) that is built from the Rust code in the `rholang-parser` crate.
2. When the plugin is loaded, it extracts this native library to a temporary directory and loads it using JNI.
3. The Java code in `RholangParserJNI.java` provides methods that call into the native library:
   - `isValid(String code)`: Checks if the given code is valid Rholang
   - `parse(String code)`: Parses the given code and returns a parse tree

### Building the Native Library

If you're developing the plugin, you'll need to build the native library before building the plugin:

1. Make sure you have Rust and Cargo installed
2. Run `make build-rholang-parser` to build the native library
3. Then run `make build-plugin` to build the plugin with the native library included

Alternatively, you can run `make build-plugin` directly, which will build the native library first.

### Troubleshooting JNI Issues

If you encounter issues with the JNI integration:

1. Check that the native library was built successfully
2. Look for JNI-related error messages in the IDE log (Help > Show Log in Explorer/Finder)
3. Make sure you're using a compatible Java version (Java 17 is recommended)

## Feedback

If you have any feedback or suggestions for improving the plugin, please open an issue on the GitHub repository.
