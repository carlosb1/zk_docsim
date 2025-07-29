//import init, { verify_receipt } from "./wasm_verifier.js";

var count = 0;
async function run() {
    const output = document.getElementById("output");
    output.textContent = count;
    count+=1;

}
/* loading run function */
window.run = run;
