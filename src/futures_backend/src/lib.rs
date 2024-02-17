#[macro_use]
extern crate serde;
use candid::{CandidType, Principal};
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_cdk::api::{caller, call::call};
use ic_ledger_types::{
    transfer, AccountIdentifier, BlockIndex, Memo, Tokens, TransferArgs, DEFAULT_FEE,
    DEFAULT_SUBACCOUNT, MAINNET_LEDGER_CANISTER_ID,
};

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
use ic_cdk::storage;

// Define the memory manager
type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

//initialize the memory manager
#[ic_cdk::init]
async fn init() -> () {
    // Store the deployer's principal at initialization
    let deployer = caller();
    storage::stable_save((b"deployer", &deployer)).unwrap();
}


//define enum for contract status
#[derive(candid::CandidType, Deserialize, Serialize, Clone, Debug, PartialEq)]
enum ContractStatus {
    Created,
    Pending,
    InProgress,
    Completed,
    Terminated,
}

//define the struct for the farmer
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct Farmer {
    name: String,
    email: Option<String>,
    phone_number: String,
    croptypes: Vec<String>,
    size_of_land: u64,
    location: String,
    national_id: String,
    created_at: u64,
    updated_at: Option<u64>,
    id: u64,
    principal: Principal,
}

// a trait to implement the farmer struct that is stored in a stable struct
impl Storable for Farmer {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

//a trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Farmer {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

//define the struct for the farmer payload
#[derive(candid::CandidType, Deserialize, Serialize, Clone, Default, Debug)]
struct FarmerPayload {
    name: String,
    email: Option<String>,
    phone_number: String,
    croptypes: Vec<String>,
    size_of_land: u64,
    location: String,
    national_id: String,
}

// define a struct for the buyers(people who buy the crops)
#[derive(candid::CandidType, Deserialize, Serialize, Clone, Debug)]
struct Buyer {
    name: String,
    email: Option<String>,
    phone_number: String,
    organization: Option<String>,
    id: u64,
    location: String,
    principal: Principal,
}

//a trait that must be implemented for the buyer struct that is stored in a stable struct
impl Storable for Buyer {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

//a trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Buyer {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

//define the struct for the buyer payload
#[derive(candid::CandidType, Deserialize, Serialize, Clone, Default, Debug)]
struct BuyerPayload {
    name: String,
    email: Option<String>,
    phone_number: String,
    organization: Option<String>,
    location: String,
}

//define the struct for the cropcontract
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
struct FutureContract {
    crop: String,
    expected_yield: u64,
    price_per_unit: u64,
    terms_and_conditions: String,
    bargain: Option<String>,
    expected_month_of_harvest: String,
    contract_status: ContractStatus,
    id: u64,
    buyer_accepted: bool,
    farmer_accepted: bool,
    farmer: Option<Principal>,
    buyer: Option<Principal>,
    short_position_holder: Option<Principal>,
    long_position_holder: Option<Principal>,
}

//a trait that must be implemented for the cropcontract struct that is stored in a stable struct
impl Storable for FutureContract {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for FutureContract {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

//define the struct for the cropcontract payload
#[derive(candid::CandidType, Deserialize, Serialize, Clone, Debug)]
struct FutureContractPayload {
    crop: String,
    expected_yield: u64,
    price_per_unit: u64,
    terms_and_conditions: String,
    expected_month_of_harvest: String,
    contract_status: ContractStatus,
}

thread_local! {
static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

static STORAGE: RefCell<StableBTreeMap<u64, Farmer, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static FURURE_CONTRACT_STORAGE: RefCell<StableBTreeMap<u64, FutureContract, Memory>> =
    RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static BUYER_STORAGE: RefCell<StableBTreeMap<u64, Buyer, Memory>> =
    RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
 }

// get the farmer by id
#[ic_cdk::query]
fn get_farmer(id: u64) -> Result<Farmer, String> {
    match_get_farmer(&id).ok_or_else(|| format!("Farmer with id={} not found", id))
}

//get farmer by principal
#[ic_cdk::query]
fn get_farmer_by_principal(principal: Principal) -> Result<Farmer, String> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .find(|farmer| farmer.principal == principal)
            .ok_or_else(|| format!("Farmer with principal={} not found", principal))
    })
}

//get all farmers
#[ic_cdk::query]
fn get_all_farmers() -> Vec<Farmer> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .collect()
    })
}

// create a new farmer
#[ic_cdk::update]
fn add_farmer(payload: FarmerPayload) -> Option<Farmer> {
    let id = ID_COUNTER.with(|counter| {
        let counter_value = *counter.borrow().get();
        let _ = counter.borrow_mut().set(counter_value + 1);
        counter_value
    });
    let farmer = Farmer {
        name: payload.name,
        email: None,
        phone_number: payload.phone_number,
        croptypes: payload.croptypes,
        size_of_land: payload.size_of_land,
        location: payload.location,
        national_id: payload.national_id,
        created_at: time(),
        updated_at: None,
        id,
        principal: ic_cdk::caller(),
    };
    do_insert_farmer(&farmer);
    Some(farmer)
}

//helper method to perform the insert operation
fn do_insert_farmer(farmer: &Farmer) {
    STORAGE.with(|service| service.borrow_mut().insert(farmer.id, farmer.clone()));
}

// a helper method to get a message by id. used in get_message
fn match_get_farmer(id: &u64) -> Option<Farmer> {
    STORAGE.with(|service| service.borrow().get(id))
}

//get buyer by id
#[ic_cdk::query]
fn get_buyer(id: u64) -> Result<Buyer, String> {
    match_get_buyer(&id).ok_or_else(|| format!("Buyer with id={} not found", id))
}

//get all buyers
#[ic_cdk::query]
fn get_all_buyers() -> Vec<Buyer> {
    BUYER_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .collect()
    })
}

//get buyer by principal
#[ic_cdk::query]
fn get_buyer_by_principal(principal: Principal) -> Result<Buyer, String> {
    BUYER_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .find(|buyer| buyer.principal == principal)
            .ok_or_else(|| format!("Buyer with principal={} not found", principal))
    })
}
//create a new buyer
#[ic_cdk::update]
fn add_buyer(payload: BuyerPayload) -> Option<Buyer> {
    let id = ID_COUNTER.with(|counter| {
        let counter_value = *counter.borrow().get();
        let _ = counter.borrow_mut().set(counter_value + 1);
        counter_value
    });
    let buyer = Buyer {
        name: payload.name,
        email: None,
        phone_number: payload.phone_number,
        organization: None,
        location: payload.location,
        id,
        principal: ic_cdk::caller(),
    };
    do_insert_buyer(&buyer);
    Some(buyer)
}

// a helper method to get a buyer by id. used in get_buyer
fn match_get_buyer(id: &u64) -> Option<Buyer> {
    BUYER_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to perform the insert operation
fn do_insert_buyer(buyer: &Buyer) {
    BUYER_STORAGE.with(|service| service.borrow_mut().insert(buyer.id, buyer.clone()));
}

//get Future contract by id
#[ic_cdk::query]
fn get_future_contract(id: u64) -> Result<FutureContract, String> {
    match_get_future_contract(&id).ok_or_else(|| format!("FutureContract with id={} not found", id))
}

//get all Future contracts
#[ic_cdk::query]
fn get_all_future_contracts() -> Vec<FutureContract> {
    FURURE_CONTRACT_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .collect()
    })
}

//get Future contracts by contract status
#[ic_cdk::query]
fn get_contracts_by_contract_status(contract_status: ContractStatus) -> Vec<FutureContract> {
    FURURE_CONTRACT_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .filter(|contract| contract.contract_status == contract_status)
            .collect()
    })
}

//get Future contracts by farmer
#[ic_cdk::query]
 fn get_contracts_by_farmer(farmer: Principal) -> Vec<FutureContract> {
    FURURE_CONTRACT_STORAGE.with(|service| service.borrow().iter().map(|(_key, value)| value.clone()).filter(|contract| contract.farmer == Some(farmer)).collect())
}

//get Future contracts by buyer
#[ic_cdk::query]
fn get_contracts_by_buyer(buyer: Principal) -> Vec<FutureContract> {
    FURURE_CONTRACT_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_key, value)| value.clone())
            .filter(|contract| contract.buyer == Some(buyer))
            .collect()
    })
}

//create a new Future contract
#[ic_cdk::update]
fn create_future_contract(payload: FutureContractPayload) -> Option<FutureContract> {
    let id = ID_COUNTER.with(|counter| {
        let counter_value = *counter.borrow().get();
        let _ = counter.borrow_mut().set(counter_value + 1);
        counter_value
    });
    let future_contract = FutureContract {
        crop: payload.crop,
        bargain: None,
        expected_yield: payload.expected_yield,
        price_per_unit: payload.price_per_unit,
        terms_and_conditions: payload.terms_and_conditions,
        expected_month_of_harvest: payload.expected_month_of_harvest,
        contract_status: ContractStatus::Created,
        buyer: None,
        farmer: None,
        id,
        short_position_holder: None,
        long_position_holder: None,
        farmer_accepted: false,
        buyer_accepted: false,
    };
    do_insert_future_contract(&future_contract);
    Some(future_contract)
}

// a hellper method to match the Future contract
fn match_get_future_contract(id: &u64) -> Option<FutureContract> {
    FURURE_CONTRACT_STORAGE.with(|service| service.borrow().get(id))
}

//helper method to perform the insert operation
fn do_insert_future_contract(future_contract: &FutureContract) {
    FURURE_CONTRACT_STORAGE.with(|service| {
        service
            .borrow_mut()
            .insert(future_contract.id, future_contract.clone())
    });
}

//method to accept the contract by the farmer
#[ic_cdk::update]
async fn claim_short_position(id: u64, bargain: Option<String>) -> Result<FutureContract, String> {

    let amount = Tokens::from_e8s(1_000_000);
    let mut future_contract = match_get_future_contract(&id)
        .ok_or_else(|| format!("FutureContract with id={} not found", id))?;
    if future_contract.short_position_holder.is_some() {
        return Err("Short position already claimed".to_string());
    }
    if future_contract.long_position_holder.is_some() {
        future_contract.contract_status = ContractStatus::InProgress;
    } else {
        future_contract.contract_status = ContractStatus::Pending;
    }
    future_contract.farmer = Some(ic_cdk::caller());
    future_contract.short_position_holder = Some(ic_cdk::caller());
    future_contract.bargain = Some(bargain.unwrap_or("".to_string()));
    do_insert_future_contract(&future_contract);
    // Call the async function to transfer margin
    transfer_margin(amount).await;
    Ok(future_contract)
}

//method to accept the contract by the buyer
#[ic_cdk::update]
async fn claim_long_position(id: u64, bargain: Option<String>) -> Result<FutureContract, String> {
    let amount = Tokens::from_e8s(1_000_000);
    let mut future_contract = match_get_future_contract(&id)
        .ok_or_else(|| format!("FutureContract with id={} not found", id))?;
    if future_contract.long_position_holder.is_some() {
        return Err("Long position already claimed".to_string());
    }
    if future_contract.short_position_holder.is_some() {
        future_contract.contract_status = ContractStatus::InProgress;
    } else {
        future_contract.contract_status = ContractStatus::Pending;
    }
    future_contract.buyer = Some(ic_cdk::caller());
    future_contract.long_position_holder = Some(ic_cdk::caller());
    future_contract.bargain = Some(bargain.unwrap_or("".to_string()));
    do_insert_future_contract(&future_contract);
    // Call the async function to transfer margin
    transfer_margin(amount).await;
    Ok(future_contract)
}

// get principal
#[ic_cdk::query]
fn get_principal() -> Principal {
    ic_cdk::caller()
}


async fn transfer_margin(amount: Tokens) -> Result<BlockIndex, String> {
     // Retrieve the deployer's principal from stored state
     let deployer: Principal = storage::stable_restore().map_err(|err| format!("Failed to retrieve deployer's principal: {:?}", err))?;

    // Construct the transfer arguments
    let transfer_args = TransferArgs {
        memo: Memo(0), // Memo can be set to any value
        amount,
        fee: DEFAULT_FEE, // Use the default fee or customize as needed
        from_subaccount: None, // Since this is a transfer from the canister, set from_subaccount to None
        to: AccountIdentifier::new(&deployer, &DEFAULT_SUBACCOUNT), // Set the recipient's principal and default subaccount
        created_at_time: None, // Optional parameter
    };

    // Perform the transfer
    let result = ic_ledger_types::transfer(
        MAINNET_LEDGER_CANISTER_ID, // The ledger canister ID
        transfer_args, // Transfer arguments
    )
    .await;

    // Handle the result of the transfer
    match result {
        Ok(block_index) => Ok(block_index),
        Err(err) => Err(format!("Failed to transfer tokens: {:?}", err)),
    }
}


// a hellper method to transfer tokens after the contract has been accepted
// async fn transfer_margin() -> BlockIndex {
//     // Retrieve the deployer's principal from stored state
//     let deployer: Principal = storage::stable_restore().unwrap();

//     transfer(
//         MAINNET_LEDGER_CANISTER_ID,
//         TransferArgs {
//             memo: Memo(0),
//             amount: Tokens::from_e8s(1_000_000),
//             fee: DEFAULT_FEE,
//             from_subaccount: None,
//             to: AccountIdentifier::new(&deployer, &DEFAULT_SUBACCOUNT),
//             created_at_time: None,
//         }
//     )
//     .await
//     .expect("call to ledger failed")
//     .expect("transfer failed")
// }

ic_cdk::export_candid!();
