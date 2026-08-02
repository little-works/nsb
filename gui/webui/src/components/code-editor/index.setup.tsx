import { __render } from '@/shared/helpter';
import { indentWithTab } from '@codemirror/commands';
import { javascript } from '@codemirror/lang-javascript';
import { json, jsonParseLinter } from '@codemirror/lang-json';
import { syntaxHighlighting } from '@codemirror/language';
import { linter } from '@codemirror/lint';
import { Compartment, EditorState } from '@codemirror/state';
import { oneDarkHighlightStyle } from '@codemirror/theme-one-dark';
import { EditorView, keymap } from '@codemirror/view';
import { basicSetup } from 'codemirror';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';

export type CodeEditorLanguage = 'javascript' | 'json';

export interface CodeEditorProps {
  value: string;
  language?: CodeEditorLanguage;
  onChange?: (value: string) => void;
}

const props = withDefaults(defineProps<CodeEditorProps>(), {
  language: 'json',
  onChange: () => {},
});

const container = ref<HTMLElement | null>(null);
const languageCompartment = new Compartment();
const themeCompartment = new Compartment();
let editor: EditorView | undefined;
let themeObserver: MutationObserver | undefined;
let synchronizing = false;

const appTheme = EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: 'var(--surface-container-lowest)',
    color: 'var(--on-surface)',
  },
  '.cm-content': {
    caretColor: 'var(--primary)',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
    fontSize: '0.875rem',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--surface-container-low)',
    borderRight: '1px solid var(--outline-variant)',
    color: 'var(--on-surface-variant)',
  },
  '.cm-activeLine, .cm-activeLineGutter': {
    backgroundColor: 'var(--surface-container)',
  },
  '.cm-focused': {
    outline: 'none',
  },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
    backgroundColor: 'var(--primary-container)',
  },
});

function isDarkTheme() {
  const theme = document.documentElement.dataset.nsbTheme;
  return (
    theme === 'dark' ||
    (theme === 'auto' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches)
  );
}

function themeExtensions() {
  return isDarkTheme()
    ? [
        EditorView.theme({}, { dark: true }),
        appTheme,
        syntaxHighlighting(oneDarkHighlightStyle),
      ]
    : [appTheme];
}

function languageExtensions() {
  return props.language === 'javascript'
    ? javascript()
    : [json(), linter(jsonParseLinter())];
}

function updateTheme() {
  editor?.dispatch({
    effects: themeCompartment.reconfigure(themeExtensions()),
  });
}

function updateLanguage() {
  editor?.dispatch({
    effects: languageCompartment.reconfigure(languageExtensions()),
  });
}

function updateEditor(value: string) {
  if (!editor || value === editor.state.doc.toString()) {
    return;
  }

  synchronizing = true;
  editor.dispatch({
    changes: { from: 0, to: editor.state.doc.length, insert: value },
  });
  synchronizing = false;
}

watch(() => props.value, updateEditor);
watch(() => props.language, updateLanguage);

onMounted(() => {
  if (!container.value) {
    return;
  }

  editor = new EditorView({
    parent: container.value,
    state: EditorState.create({
      doc: props.value,
      extensions: [
        basicSetup,
        keymap.of([indentWithTab]),
        EditorView.lineWrapping,
        languageCompartment.of(languageExtensions()),
        themeCompartment.of(themeExtensions()),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !synchronizing) {
            props.onChange?.(update.state.doc.toString());
          }
        }),
      ],
    }),
  });

  themeObserver = new MutationObserver(updateTheme);
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-nsb-theme'],
  });
});

onBeforeUnmount(() => {
  themeObserver?.disconnect();
  editor?.destroy();
});

defineOptions({ name: 'CodeEditor' });

export default __render<CodeEditorProps>(() => (
  <div ref={container} class="h-full min-h-0" />
));
