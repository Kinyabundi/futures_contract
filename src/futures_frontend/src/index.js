
//interner identity
import {
  createActor,
  cropcontract_backend,
} from "../../declarations/futures_backend";
import { AuthClient } from "@dfinity/auth-client";
import { HttpAgent } from "@dfinity/agent";
let actor = cropcontract_backend;
console.log(process.env.CANISTER_ID_INTERNET_IDENTITY);


const loginButton = document.getElementById("login");
loginButton.onclick = async (e) => {
  e.preventDefault();
  let authClient = await AuthClient.create();
  // start the login process and wait for it to finish
  await new Promise((resolve) => {
    authClient.login({
      identityProvider:
        process.env.DFX_NETWORK === "ic"
          ? "https://identity.ic0.app"
          : `http:/bd3sg-teaaa-aaaaa-qaaba-cai/.localhost:4943`,
      onSuccess: resolve,
    });
  });
  const identity = authClient.getIdentity();
  const agent = new HttpAgent({ identity });
  actor = createActor(process.env.CANISTER_ID_CROPCONTRACT_BACKEND, {
    agent,
  });
  
  console.log(actor);

  // Emit events
  loginEmitter.emit('loginEvent', 'Login was successful!');


  // backend functions
  // get_principal
  console.log("Get principal:");
  const get_principal = await actor.get_principal();
  console.log(get_principal);
  // document.getElementById("output_test").innerHTML = get_principal;

};