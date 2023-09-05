Build wasm using
```shell
wasm-pack build --target web .
```

The `resources` folder must be in the `pkg` folder to be able to run it.

`index.html` must be in the pkg with this body:
```html
<body id="wasm-example">
  <script type="module">
    import init from "./idk.js";
    init()
    .then(() => {
        console.log("WASM Loaded");
    });
  </script>
</body>
```