import * as mjolnir from "mjolnir";
import * as axios from "axios";

let private_key = null;
let public_key = null;
let user_address = null;
let user_address_hex = "";
let fee = null;

let utxos = [];
let inputs = [];
let outputs = [];
let accounts = [];

window.document.getElementById("store_pk").onclick = function(self) {
    console.log("loading private key..")
    let bech32 = document.getElementById("private_key_hex").value;
    private_key = mjolnir.PrivateKey.from_bench32(bech32);
    console.log("extracting public key..")
    let public_key = private_key.public();
    document.getElementById("public_key_hex").value = public_key.to_hex();
    console.log("extracting address key..")
    user_address = public_key.address();
    user_address_hex = user_address.to_bech32();
    document.getElementById("user_address").value = user_address.to_bech32();
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
      if (input.type === "utxo") {
         console.log("creating transaction id from", input.utxo.in_txid);
         var tx = mjolnir.TransactionId.from_hex(input.utxo.in_txid);
         console.log("creating utxo pointer");
         var utxo = mjolnir.UtxoPointer.new(tx, input.utxo.in_idx, BigInt(input.out_value));
         console.log("creating utxo input");
         var input = mjolnir.Input.from_utxo(utxo);
         console.log("adding utxo input");
         txbuilder.add_input(input);
         console.log("input added");
      } else if (input.type === "account") {
         console.log("creating account input", input.account.account);
         var account = input.account.account;
         var input = mjolnir.Input.from_account(account,
               BigInt(input.out_value));
         txbuilder.add_input(input);
      } else {
         console.log("unknown input type", input.type);
      }
   });
   console.log("Adding outputs:");
   outputs.forEach(function (output, i) {
      console.log("  output: ", output);
      var address = mjolnir.Address.from_bech32(output.address);
      txbuilder.add_output(address, output.value)
   });
   console.log("Creating fee");
   var fee = createFee();
   console.log("Fee created", fee);
   console.log("Policy");
   var address = mjolnir.Address.from_bech32(
         document.getElementById("input_policy_address").value);
   var policy = mjolnir.OutputPolicy.one(address);
   console.log("Policy created", policy);
   var finalizer = txbuilder.finalize(fee, policy).get_result();
   console.log("Adding signatures to:", finalizer);
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

window.document.getElementById("estimate_btn").onclick = function(self) {
   show_balance();
}

window.document.getElementById("btn_add_account").onclick = function(self) {
   var pk = mjolnir.PrivateKey.from_bench32(document.getElementById("input_account_pk").value.trim());
   var pub = pk.public();
   var account = mjolnir.Account.from_public(pub);
   accounts.push({'private':pk, 'public': pub, 'account': account});
   show_accounts();
}

window.document.getElementById("btn_add_account_input").onclick = function(self) {
   var i = parseInt(document.getElementById("sel_input_account").value);
   var account = accounts[i];
   var value = parseInt(document.getElementById("input_account_value").value);
   inputs.push({"type":"account", "account": account, "out_value": value});
   show_inputs();
}

window.document.getElementById("btn_deserialize").onclick = function(self) {
   console.log("reading content..");
   var t = document.getElementById("transaction_input").value.trim();
   console.log("creating transaction from ", t);
   var tx = mjolnir.SignedTransaction.from_hex(t);
   console.log("deserializing...");
   document.getElementById("transaction_description").textContent = JSON.stringify(tx.describe());
}

function refresh() {
   axios.get("http://localhost:8443/api/v0/utxo",{})
         .then( result => {
            console.log("downloaded new data", result.data);
            filter_utxos(result.data)
            show_utxos();
   });
}

function createFee() {
  var a = parseInt(document.getElementById("input_fee_a").value);
  var b = parseInt(document.getElementById("input_fee_b").value);
  return mjolnir.Fee.linear_fee(BigInt(a), BigInt(b));
}

function show_balance() {
   console.log("Creating a builder.");
   var txbuilder = mjolnir.TransactionBuilder.new();
   console.log("Adding inputs:");
   inputs.forEach(function (input, i) {
      console.log("  input: ", input);
      if (input.type === "utxo") {
         console.log("creating transaction id from", input.utxo.in_txid);
         var tx = mjolnir.TransactionId.from_hex(input.utxo.in_txid);
         console.log("creating utxo pointer");
         var utxo = mjolnir.UtxoPointer.new(tx, input.utxo.in_idx, BigInt(input.out_value));
         console.log("creating utxo input");
         var input = mjolnir.Input.from_utxo(utxo);
         console.log("adding utxo input");
         txbuilder.add_input(input);
         console.log("input added");
      } else if (input.type === "account") {
         console.log("creating account input", input.account.account);
         var account = input.account.account;
         var input = mjolnir.Input.from_account(account,
               BigInt(input.out_value));
         txbuilder.add_input(input);
      } else {
         console.log("unknown input type", input.type);
      }
   })
   console.log("Adding outputs:");
   outputs.forEach(function (output, i) {
      console.log("  output: ", output);
      var address = mjolnir.Address.from_bech32(output.address);
      txbuilder.add_output(address, output.value)
   });
   console.log("Creating fee");
   var fee = createFee();
   console.log("Fee created", fee);
   console.log("Policy");
   var address = mjolnir.Address.from_bech32(
         document.getElementById("input_policy_address").value);
   var policy = mjolnir.OutputPolicy.one(address);
   console.log("Policy created", policy);
   var balance = txbuilder.get_balance(fee);
   document.getElementById("output_balance").textContent = balance.get_sign() + ' ' + balance.get_value().as_u64();
   var estimated_fee = txbuilder.estimate_fee(fee);
   document.getElementById("output_estimated_fee").textContent = estimated_fee.as_u64();
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
           inputs.push({"type":"utxo", "utxo":utxo, "out_value": utxo.out_value});
           show_inputs();
        };
        utxo_block.appendChild(li);
    });
}

function show_inputs() {
 
    var input_block = $("#tbl_input > tbody").empty();
    var total = 0;
    inputs.forEach(function(input, i) {
        console.log(input);
        if (input.type == "utxo") {
           var address = input.utxo.in_txid + " " + input.utxo.in_idx;
        } else {
           var address = input.account.public.to_hex();
        }
        input_block.append(
            $("<tr>").append($("<td>").text(input.type))
                     .append($("<td>").text(address))
                     .append($("<tx>").text(input.out_value))
        );
        total += parseInt(input.out_value);
    })

    document.getElementById("tx_input_total").textContent=total;
}

function filter_utxos(new_utxos) {
   utxos = new_utxos.filter(function(utxo) {
     return (utxo.out_addr === user_address_hex);
   })
}

function show_outputs() {
    var output_block = $("#tbl_output > tbody").empty();
    var total = 0;
    outputs.forEach(function(output, i) {
        console.log(output);
        output_block.append(
            $("<tr>").append($("<td>").text(""+output.address))
                     .append($("<td>").text(""+output.value))
        );
        total += parseInt(output.value);
    })
    document.getElementById("tx_output_total").textContent=total;
}

function show_accounts() {
    var account_select = document.getElementById("sel_input_account");
    var output_block = document.getElementById("account_block");
    clear_el(output_block);
    clear_el(account_select);
    accounts.forEach(function(account, i) {
        var li = document.createElement("li");
        var a = document.createElement("a");
        var k = account.account.to_bech32();
        var t = document.createTextNode("" +
                 k.substr(0,3) + "..." + k.substr(k.length - 3))
        li.appendChild(a);
        a.appendChild(t);
        output_block.appendChild(li);
        var option = document.createElement("option");
        option.textContent = account.account.to_bech32();
        option.value = i;
        account_select.appendChild(option);
    })
}

function clear_el(block) {
    while (block.hasChildNodes()) {
        block.removeChild(block.lastChild);
    }
}
