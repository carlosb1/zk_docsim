import init, { verify_receipt } from "./wasm_verifier.js";


async function hashReceipt(receiptBytes) {
    const hashBuffer = await crypto.subtle.digest("SHA-256", receiptBytes.buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    const output = document.getElementById("output");
    output.textContent += hex ;
    return hex;
}

async function run() {
    const output = document.getElementById("output");
    output.textContent = "Verificando...";

    await init(); // Carga el WASM

    const response = await fetch("receipt.bin");
    const receiptBytes = new Uint8Array(await response.arrayBuffer());
    await hashReceipt(receiptBytes);


    // ID que debe coincidir con el usado en el backend
    const imageId = [
        4239518147, 485871892, 529799477, 3277490305,
        2515661234, 362053723, 2729086396, 2562830873
    ];

    try {
        verify_receipt(receiptBytes, imageId);
        output.textContent += "\n✅ Verificado correctamente.";
    } catch (e) {
        output.textContent += "\n❌ Error: " + e;
    }
}
/* loading run function */
window.run = run;
