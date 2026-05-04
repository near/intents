(component
  (core module $m
    (func (export "run") (result i32)
      i32.const 0))
  (core instance $i (instantiate $m))
  (func $run (result (result))
    (canon lift (core func $i "run")))
  (instance $iface
    (export "run" (func $run)))
  (export "wasi:cli/run@0.2.6" (instance $iface))
)