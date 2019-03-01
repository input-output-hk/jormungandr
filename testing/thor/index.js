import * as mjolnir from "mjolnir";
import * as axios from "axios";

let private_key = null;
let public_key = null;
let user_address = null;
let user_address_hex = "";

let utxos = [];
let inputs = [];
let outputs = [];

window.document.getElementById("store_pk").onclick = function(self) {
    console.log("loading private key..")
    let hex = document.getElementById("private_key_hex").value;
    private_key = mjolnir.PrivateKey.from_hex(hex);
    console.log("extracting public key..")
    let public_key = private_key.public();
    document.getElementById("public_key_hex").value = public_key.to_hex();
    console.log("extracting address key..")
    user_address = public_key.address();
    user_address_hex = user_address.to_betch32();
    document.getElementById("user_address").value = user_address.to_betch32();
};

window.document.getElementById("tx_add").onclick = function(self) {
    var out_addr = document.getElementById("tx_out_address").value;
    var out_value = document.getElementById("tx_out_value").value;
    outputs.push({"address":out_addr, "value": out_value});
    show_outputs();
}

window.document.getElementById("sign_btn").onclick = function(self) {
   console.log("Creating a builder.");
   var txbuilder = mjolnir.TransactionBuilder.new();
   console.log("Adding inputs:");
   inputs.forEach(function (input, i) {
      console.log("  input: ", input);
      var tx = mjolnir.TransactionId.from_hex(input.in_txid);
      var utxo = mjolnir.UtxoPointer.new(tx, input.in_idx, BigInt(input.out_value));
      console.log("  utxo: ", utxo);
      txbuilder.add_input(utxo);
   })
   console.log("Adding outputs:");
   outputs.forEach(function (output, i) {
      console.log("  output: ", output);
      var address = mjolnir.Address.from_betch32(output.address);
      txbuilder.add_output(address, output.value)
   });
   var finalizer = txbuilder.finalize();
   console.log("Adding outputs:");
   inputs.forEach(function (input, i) {
       console.log(private_key);
       finalizer.sign(private_key);
   })
   var signed_tx = finalizer.build();
   document.getElementById("signed_tx_result").textContent = signed_tx.to_hex();
}

window.document.getElementById("clear_tx_btn").onclick = function(self) {
   inputs = [];
   outputs = [];
   show_inputs();
   show_outputs();
}

window.document.getElementById("refresh_btn").onclick = function(self) {
   refresh();
}

window.document.getElementById("post_btn").onclick = function(self) {
    var tx = document.getElementById("signed_tx_result").textContent;
    var bin = new Buffer(tx, 'hex')
    var xhr = new XMLHttpRequest();
    xhr.open('POST', 'http://localhost:8443/api/v0/transaction');
    xhr.setRequestHeader('Content-Type', 'application/octet-stream');
    xhr.send(bin);
}

function refresh() {
   axios.get("http://localhost:8443/api/v0/utxo",{})
         .then( result => {
            console.log("downloaded new data", result.data);
            filter_utxos(result.data)
            show_utxos();
   });
}

function show_utxos() {
    var utxo_block = document.getElementById("utxo_block");
    while (utxo_block.hasChildNodes()) {
       utxo_block.removeChild(utxo_block.lastChild);
    }
    utxos.forEach(function(utxo, i) {
        var li = document.createElement("li");
        var a = document.createElement("a")
        var t = document.createTextNode(utxo.out_value + " "
              + utxo.in_txid.substr(0,3) + "..."
              + utxo.in_txid.substr(utxo.in_txid.length - 3)
              + ":" + utxo.in_idx
        );
        li.appendChild(a);
        a.appendChild(t);
        a.setAttribute("class","clickable");
        a.onclick = function() { 
           inputs.push(utxo);
           show_inputs();
        };
        utxo_block.appendChild(li);
    });
}

function show_inputs() {
    var input_block = document.getElementById("tx_input_block");
    clear_el(input_block);
    var total = 0;
    inputs.forEach(function(input, i) {
        var li = document.createElement("li");
        var a = document.createElement("a")
        var t = document.createTextNode(
              [input.out_value
              ,input.in_txid
              ,input.in_idx
              ].join(" "));
        total += parseInt(input.out_value);
        li.appendChild(a);
        a.appendChild(t);
        input_block.appendChild(li);
    })
    document.getElementById("tx_input_total").textContent=total;
}

function filter_utxos(new_utxos) {
   utxos = new_utxos.filter(function(utxo) {
     return (utxo.out_addr === user_address_hex);
   })
}

function show_outputs() {
    var output_block = document.getElementById("tx_output_block");
    var total = 0;
    clear_el(output_block);
    outputs.forEach(function(output, i) {
        console.log(output);
        var li = document.createElement("li");
        var a = document.createElement("a")
        var t = document.createTextNode("" + output.value + " " + output.address);
        total += parseInt(output.value);
        li.appendChild(a);
        a.appendChild(t);
        output_block.appendChild(li);
    })
    document.getElementById("tx_output_total").textContent=total;
}



function clear_el(block) {
    while (block.hasChildNodes()) {
        block.removeChild(block.lastChild);
    }
}
