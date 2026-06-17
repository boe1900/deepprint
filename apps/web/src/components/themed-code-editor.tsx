import CodeMirror from "@uiw/react-codemirror";
import {
  StreamLanguage,
  defaultHighlightStyle,
  syntaxHighlighting,
  type StreamParser,
} from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { useMemo } from "react";
import { cn } from "@/lib/utils";

type CodeEditorLanguage = "plain" | "json" | "typst";

type ThemedCodeEditorProps = {
  className?: string;
  fillHeight?: boolean;
  language?: CodeEditorLanguage;
  minHeight?: number;
  onChange: (value: string) => void;
  placeholder?: string;
  value: string;
};

type TypstParserState = {
  inBlockComment: boolean;
};

const jsonStreamLanguage = StreamLanguage.define({
  token(stream) {
    if (stream.eatSpace()) return null;
    if (stream.match(/"(?:[^"\\]|\\.)*"?/)) {
      const rest = stream.string.slice(stream.pos);
      return /^\s*:/.test(rest) ? "propertyName" : "string";
    }
    if (stream.match(/-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/)) {
      return "number";
    }
    if (stream.match(/\b(?:true|false|null)\b/)) return "atom";
    if (stream.match(/[{}[\],:]/)) return "punctuation";
    stream.next();
    return "invalid";
  },
  tokenTable: {
    atom: tags.atom,
    invalid: tags.invalid,
    number: tags.number,
    propertyName: tags.propertyName,
    punctuation: tags.punctuation,
    string: tags.string,
  },
} satisfies StreamParser<unknown>);

const typstStreamLanguage = StreamLanguage.define({
  startState: () => ({ inBlockComment: false }),
  token(stream, state: TypstParserState) {
    if (state.inBlockComment) {
      if (stream.skipTo("*/")) {
        stream.match("*/");
        state.inBlockComment = false;
      } else {
        stream.skipToEnd();
      }
      return "comment";
    }

    if (stream.eatSpace()) return null;
    if (stream.match("/*")) {
      state.inBlockComment = true;
      return "comment";
    }
    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.sol() && stream.match(/=+\s+/)) {
      stream.skipToEnd();
      return "heading";
    }
    if (stream.match(/#[A-Za-z_][\w-]*/)) return "keyword";
    if (stream.match(/"(?:[^"\\]|\\.)*"?/)) return "string";
    if (stream.match(/\b(?:true|false|auto|none)\b/)) return "atom";
    if (stream.match(/\b\d+(?:\.\d+)?(?:pt|mm|cm|in|em|fr|%)?\b/)) {
      return "number";
    }
    if (stream.match(/[@$][A-Za-z_][\w-]*/)) return "variableName";
    if (stream.match(/[{}[\](),.;:]/)) return "punctuation";
    if (stream.match(/[+\-*/=<>!]+/)) return "operator";
    stream.next();
    return null;
  },
  tokenTable: {
    atom: tags.atom,
    heading: tags.heading,
    keyword: tags.keyword,
    number: tags.number,
    operator: tags.operator,
    punctuation: tags.punctuation,
    string: tags.string,
    variableName: tags.variableName,
  },
} satisfies StreamParser<TypstParserState>);

const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "transparent",
    color: "var(--foreground)",
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    fontSize: "13px",
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    overflow: "hidden",
  },
  ".cm-editor": {
    height: "100%",
    minHeight: 0,
  },
  ".cm-scroller": {
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    lineHeight: "1.68",
    overflow: "auto !important",
    minHeight: 0,
    scrollbarGutter: "stable",
  },
  ".cm-content": {
    caretColor: "var(--foreground)",
    minHeight: "100%",
    padding: "12px 0",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "var(--foreground)",
  },
  ".cm-gutters": {
    backgroundColor: "color-mix(in srgb, var(--muted) 54%, transparent)",
    color: "var(--muted-foreground)",
    borderRight: "1px solid color-mix(in srgb, var(--border) 72%, transparent)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "color-mix(in srgb, var(--muted) 72%, transparent)",
    color: "var(--foreground)",
  },
  ".cm-activeLine": {
    backgroundColor: "color-mix(in srgb, var(--muted) 36%, transparent)",
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection": {
    backgroundColor: "color-mix(in srgb, var(--ring) 26%, transparent)",
  },
  ".cm-placeholder": {
    color: "var(--muted-foreground)",
  },
  ".cm-focused": {
    outline: "none",
  },
});

export function ThemedCodeEditor({
  className,
  fillHeight = false,
  language = "plain",
  minHeight = 420,
  onChange,
  placeholder,
  value,
}: ThemedCodeEditorProps) {
  const extensions = useMemo(() => {
    const list = [
      editorTheme,
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      EditorView.lineWrapping,
    ];
    if (language === "json") list.push(jsonStreamLanguage);
    if (language === "typst") list.push(typstStreamLanguage);
    return list;
  }, [language]);

  return (
    <div
      className={cn(
        "ui-selectable flex flex-col overflow-hidden rounded-xl bg-background text-sm ring-1 ring-foreground/10 transition-[box-shadow] focus-within:ring-3 focus-within:ring-ring/50 dark:bg-input/30",
        fillHeight && "h-full min-h-0 flex-1",
        className,
      )}
      style={fillHeight ? undefined : { minHeight }}
    >
      <CodeMirror
        basicSetup={{
          autocompletion: true,
          bracketMatching: true,
          foldGutter: true,
          highlightActiveLine: true,
          highlightActiveLineGutter: true,
          lineNumbers: true,
        }}
        className={fillHeight ? "h-full min-h-0 flex-1 overflow-hidden" : undefined}
        extensions={extensions}
        height={fillHeight ? "100%" : undefined}
        maxHeight={fillHeight ? "100%" : undefined}
        minHeight={fillHeight ? undefined : `${minHeight}px`}
        onChange={onChange}
        placeholder={placeholder}
        value={value}
      />
    </div>
  );
}
