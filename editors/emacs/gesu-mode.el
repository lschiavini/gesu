;;; gesu-mode.el --- Major mode for the Gesù language -*- lexical-binding: t; -*-

;; Author: Lucas Schiavini
;; URL: https://github.com/lschiavini/gesu
;; Version: 0.1.2
;; Package-Requires: ((emacs "27.1"))
;; Keywords: languages

;;; Commentary:

;; Gesù is a programming language with two booleans: False and Jesus.
;; This mode gives .gesu files syntax colors, comments, indentation that
;; follows `amen', and the language server (`gesu lsp') through eglot or
;; lsp-mode.
;;
;;   (add-to-list 'load-path "/path/to/gesu/editors/emacs")
;;   (require 'gesu-mode)
;;   (add-hook 'gesu-mode-hook #'eglot-ensure)

;;; Code:

(defgroup gesu nil
  "Support for the Gesù language."
  :group 'languages
  :prefix "gesu-")

(defcustom gesu-indent-offset 4
  "Number of spaces for each level of indentation."
  :type 'integer
  :group 'gesu)

(defcustom gesu-command "gesu"
  "The gesu binary.  It serves the editor when run as `gesu lsp'."
  :type 'string
  :group 'gesu)

(defconst gesu-keywords
  '("let" "mortal" "law" "parable" "give" "amen" "if" "otherwise" "while"
    "repeat" "times" "for" "each" "in" "break" "continue" "tempt" "forgive"
    "sin" "may" "preach" "deny" "and" "or" "is" "not" "lend" "entrust"
    "discern" "kind" "pagan" "baptize" "as" "repent")
  "The words of the language.")

(defconst gesu-types
  '("Number" "Decimal" "Text" "Truth" "Maybe" "Pagan" "Nothing")
  "The built-in types.")

(defconst gesu-builtins
  '("length" "push" "pop" "get" "lookup" "contains" "remove" "keys" "copy"
    "range" "sort" "reverse" "split" "join" "trim" "upper" "lower" "replace"
    "chars" "text" "number" "decimal" "floor" "round" "abs" "min" "max"
    "read_file" "write_file" "read_line" "args" "parse_json" "to_json")
  "The built-in parables.")

(defvar gesu-font-lock-keywords
  `(;; There is only one Truth.
    ("\\_<\\(?:True\\|true\\|TRUE\\)\\_>" . font-lock-warning-face)
    ("\\_<\\(?:Jesus\\|False\\|Found\\|Lost\\)\\_>" . font-lock-constant-face)
    ("\\_<parable[ \t]+\\([[:alpha:]_][[:alnum:]_]*\\)" 1 font-lock-function-name-face)
    ("\\_<\\(?:let\\(?:[ \t]+mortal\\)?\\|law\\)[ \t]+\\([[:alpha:]_][[:alnum:]_]*\\)"
     1 font-lock-variable-name-face)
    (,(regexp-opt gesu-keywords 'symbols) . font-lock-keyword-face)
    (,(regexp-opt gesu-types 'symbols) . font-lock-type-face)
    (,(concat (regexp-opt gesu-builtins 'symbols) "[ \t]*(") 1 font-lock-builtin-face)
    ;; All capitals is a law; one capital starts a kind or a case.
    ("\\_<[A-Z][A-Z0-9_]+\\_>" . font-lock-constant-face)
    ("\\_<[A-Z][[:alnum:]_]*\\_>" . font-lock-type-face))
  "Highlighting for `gesu-mode'.")

(defvar gesu-mode-syntax-table
  (let ((table (make-syntax-table)))
    ;; A comment runs from `--' to the end of the line.
    (modify-syntax-entry ?- ". 12" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?_ "_" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    table)
  "Syntax table for `gesu-mode'.")

(defconst gesu--opener-re
  (concat "[ \t]*\\(?:"
          "\\_<\\(?:if\\|otherwise\\|while\\|repeat\\|for\\|tempt\\|forgive\\|discern\\|parable\\|kind\\)\\_>"
          "\\|"
          ;; A case of `discern' whose branch starts on the next line.
          "[A-Z][[:alnum:]_]*\\(?:([^)]*)\\)?:[ \t]*\\(?:--.*\\)?$"
          "\\)")
  "Matches a line after which the next line is indented.")

(defconst gesu--closer-re
  "[ \t]*\\_<\\(?:amen\\|otherwise\\|forgive\\)\\_>"
  "Matches a line that closes the block above it.")

(defun gesu--previous-code-line ()
  "Move to the previous line that is not blank.
Return nil when there is none."
  (let ((found nil))
    (while (and (not found) (zerop (forward-line -1)))
      (unless (looking-at-p "[ \t]*$")
        (setq found t)))
    found))

(defun gesu--calculate-indent ()
  "Return the column the current line should be indented to."
  (save-excursion
    (beginning-of-line)
    (let* ((case-fold-search nil)
           (closes (looking-at-p gesu--closer-re)))
      (if (not (gesu--previous-code-line))
          0
        (let ((indent (current-indentation)))
          (when (looking-at-p gesu--opener-re)
            (setq indent (+ indent gesu-indent-offset)))
          (when closes
            (setq indent (- indent gesu-indent-offset)))
          (max indent 0))))))

(defun gesu-indent-line ()
  "Indent the current line as Gesù code."
  (interactive)
  (let ((indent (gesu--calculate-indent))
        (offset (- (current-column) (current-indentation))))
    (indent-line-to indent)
    (when (> offset 0)
      (forward-char offset))))

;;;###autoload
(define-derived-mode gesu-mode prog-mode "Gesù"
  "Major mode for the Gesù language."
  :syntax-table gesu-mode-syntax-table
  (setq-local comment-start "-- ")
  (setq-local comment-start-skip "--+[ \t]*")
  (setq-local comment-end "")
  (setq-local indent-line-function #'gesu-indent-line)
  (setq-local indent-tabs-mode nil)
  (setq-local tab-width gesu-indent-offset)
  (setq-local font-lock-defaults '(gesu-font-lock-keywords nil nil)))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.gesu\\'" . gesu-mode))

;; The language server, for whichever client is in use.

(defvar eglot-server-programs)
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs `(gesu-mode . (,gesu-command "lsp"))))

(defvar lsp-language-id-configuration)
(declare-function lsp-register-client "ext:lsp-mode")
(declare-function make-lsp-client "ext:lsp-mode")
(declare-function lsp-stdio-connection "ext:lsp-mode")
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(gesu-mode . "gesu"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection (lambda () (list gesu-command "lsp")))
    :major-modes '(gesu-mode)
    :server-id 'gesu)))

(provide 'gesu-mode)

;;; gesu-mode.el ends here
