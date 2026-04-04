module.exports = function wasmPlugin() {
  return {
    name: 'wasm-plugin',
    configureWebpack() {
      return {
        experiments: {
          asyncWebAssembly: true,
        },
      };
    },
  };
};
