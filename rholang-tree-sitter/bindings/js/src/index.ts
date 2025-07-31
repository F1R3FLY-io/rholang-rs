import { Language, Parser } from "web-tree-sitter";

let lang: Language | null = null;

export default async function loadParser(): Promise<Parser> {
  if (!lang) {
    await Parser.init();
    const wasmUrl = new URL("../tree-sitter-rholang.wasm", import.meta.url);
    lang = await Language.load(wasmUrl.toString());
  }

  const parser = new Parser();
  parser.setLanguage(lang);
  return parser;
}

export { loadParser };
