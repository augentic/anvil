(component
  (type $ty-exit (instance
    (type $exit-with-code-ty (func (param "status-code" u8)))
    (export "exit-with-code" (func (type $exit-with-code-ty)))
  ))
  (import "wasi:cli/exit@0.2.0" (instance $exit (type $ty-exit)))
  (alias export $exit "exit-with-code" (func $exit-with-code))
  (core func $exit-lower (canon lower (func $exit-with-code)))

  (core module $main
    (import "exit" "exit-with-code" (func $exit-with-code (param i32)))
    (func $run (export "run") (result i32)
      i32.const 7
      call $exit-with-code
      i32.const 0))

  (core instance $exit-imports
    (export "exit-with-code" (func $exit-lower)))
  (core instance $main-instance
    (instantiate $main
      (with "exit" (instance $exit-imports))))

  (type $run-result (result))
  (type $run-ty (func (result $run-result)))
  (alias core export $main-instance "run" (core func $run-core))
  (func $run-func (type $run-ty) (canon lift (core func $run-core)))

  (component $run-shim
    (type $shim-result (result))
    (type $shim-run-ty (func (result $shim-result)))
    (import "import-func-run" (func $run (type $shim-run-ty)))
    (export "run" (func $run)))
  (instance $run-instance
    (instantiate $run-shim
      (with "import-func-run" (func $run-func))))
  (export "wasi:cli/run@0.2.0" (instance $run-instance)))
