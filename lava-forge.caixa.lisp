(defcaixa
  :name
  "lava-forge"
  :kind
  :Biblioteca
  :ecosystem
  :rust-single-crate
  :package
  {:name "lava-forge"
   :version "0.1.0"
   :description "Tatara-lisp source generator for lava providers. Consumes terraform providers schema JSON and emits typed (deflava-resource ...) forms. Same upstream that pangea-forge consumes; targets the tatara-lisp surface instead of Ruby."
   :license "MIT"
   :repository "https://github.com/pleme-io/lava-forge"}
  :ci-config
  {:bump {:default-type "patch"}
   :publish {:no-verify true}}
  :workflows
  [:auto-release :pre-merge-gate :security-gate])
