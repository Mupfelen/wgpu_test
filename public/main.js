import init, { run_app } from './pkg/my_wgpu_app.js';

async function main() {
    await init();
    run_app();
}

main();