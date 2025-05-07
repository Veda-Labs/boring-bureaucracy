use super::building_block::Actionable;
use crate::actions::admin_action::CallerType;
use crate::actions::{admin_action::AdminAction, deploy_contract_action::DeployContract};
use crate::bindings::{auth::Auth, boring_vault::BoringVault};
use crate::block_manager::shared_cache::{CacheValue, SharedCache};
use crate::utils::address_or_contract_name::{AddressOrContractName, derive_contract_address};
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::{Address, Bytes, U256, bytes};
use alloy::sol_types::{SolCall, SolConstructor};
use async_trait::async_trait;
use eyre::{Result, eyre};
use log::warn;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BoringVaultBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    boring_vault_address: Option<Address>,
    #[serde(default)]
    pub roles_authority: Option<AddressOrContractName>,
    #[serde(default)]
    roles_authority_address: Option<Address>,
    #[serde(default)]
    pub boring_vault_name: Option<String>,
    #[serde(default)]
    pub boring_vault_symbol: Option<String>,
    #[serde(default)]
    pub boring_vault_decimals: Option<u8>,
    #[serde(default)]
    pub hook: Option<AddressOrContractName>,
    #[serde(default)]
    hook_address: Option<Address>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    #[serde(default)]
    manager_address: Option<Address>,
    #[serde(default)]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    teller_address: Option<Address>,
    #[serde(default)]
    executor: Option<Address>,
}

const BORING_VAULT_BYTECODE: Bytes = bytes!(
    "0x60e06040523480156200001157600080fd5b506040516200233738038062002337833981016040819052620000349162000270565b83600084848483620000478482620003a3565b506001620000568382620003a3565b5060ff81166080524660a0526200006c6200010f565b60c0525050600680546001600160a01b038086166001600160a01b03199283168117909355600780549186169190921617905560405190915033907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e090600090a36040516001600160a01b0382169033907fa3396fd7f6e0a21b50e5089d2da70d5ac0a3bbbd1f617a93f134b7638998019890600090a3505050505050620004ed565b60007f8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f60006040516200014391906200046f565b6040805191829003822060208301939093528101919091527fc89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc660608201524660808201523060a082015260c00160405160208183030381529060405280519060200120905090565b634e487b7160e01b600052604160045260246000fd5b600082601f830112620001d357600080fd5b81516001600160401b0380821115620001f057620001f0620001ab565b604051601f8301601f19908116603f011681019082821181831017156200021b576200021b620001ab565b816040528381526020925086838588010111156200023857600080fd5b600091505b838210156200025c57858201830151818301840152908201906200023d565b600093810190920192909252949350505050565b600080600080608085870312156200028757600080fd5b84516001600160a01b03811681146200029f57600080fd5b60208601519094506001600160401b0380821115620002bd57600080fd5b620002cb88838901620001c1565b94506040870151915080821115620002e257600080fd5b50620002f187828801620001c1565b925050606085015160ff811681146200030957600080fd5b939692955090935050565b600181811c908216806200032957607f821691505b6020821081036200034a57634e487b7160e01b600052602260045260246000fd5b50919050565b601f8211156200039e57600081815260208120601f850160051c81016020861015620003795750805b601f850160051c820191505b818110156200039a5782815560010162000385565b5050505b505050565b81516001600160401b03811115620003bf57620003bf620001ab565b620003d781620003d0845462000314565b8462000350565b602080601f8311600181146200040f5760008415620003f65750858301515b600019600386901b1c1916600185901b1785556200039a565b600085815260208120601f198616915b8281101562000440578886015182559484019460019091019084016200041f565b50858210156200045f5787850151600019600388901b60f8161c191681555b5050505050600190811b01905550565b60008083546200047f8162000314565b600182811680156200049a5760018114620004b057620004e1565b60ff1984168752821515830287019450620004e1565b8760005260208060002060005b85811015620004d85781548a820152908401908201620004bd565b50505082870194505b50929695505050505050565b60805160a05160c051611e1a6200051d600039600061095901526000610924015260006102f10152611e1a6000f3fe6080604052600436106101855760003560e01c80637ecebe00116100d1578063bc197c811161008a578063dd62ed3e11610064578063dd62ed3e146104ed578063f23a6e6114610525578063f2fde38b14610551578063f6e715d01461057157600080fd5b8063bc197c8114610481578063bf7e214f146104ad578063d505accf146104cd57600080fd5b80637ecebe00146103a75780637f5a7c7b146103d45780638929565f1461040c5780638da5cb5b1461042c57806395d89b411461044c578063a9059cbb1461046157600080fd5b8063224d87031161013e5780633644e515116101185780633644e5151461032557806339d6ba321461033a57806370a082311461035a5780637a9e5e4b1461038757600080fd5b8063224d87031461029257806323b872dd146102bf578063313ce567146102df57600080fd5b806301ffc9a71461019157806306fdde03146101c6578063095ea7b3146101e8578063150b7a021461020857806318160ddd1461024c57806318457e611461027057600080fd5b3661018c57005b600080fd5b34801561019d57600080fd5b506101b16101ac3660046114e4565b610591565b60405190151581526020015b60405180910390f35b3480156101d257600080fd5b506101db6105c8565b6040516101bd919061155e565b3480156101f457600080fd5b506101b1610203366004611586565b610656565b34801561021457600080fd5b50610233610223366004611669565b630a85bd0160e11b949350505050565b6040516001600160e01b031990911681526020016101bd565b34801561025857600080fd5b5061026260025481565b6040519081526020016101bd565b34801561027c57600080fd5b5061029061028b3660046116d5565b6106c2565b005b34801561029e57600080fd5b506102b26102ad36600461177c565b610788565b6040516101bd9190611816565b3480156102cb57600080fd5b506101b16102da366004611878565b6108ff565b3480156102eb57600080fd5b506103137f000000000000000000000000000000000000000000000000000000000000000081565b60405160ff90911681526020016101bd565b34801561033157600080fd5b50610262610920565b34801561034657600080fd5b506102906103553660046116d5565b61097b565b34801561036657600080fd5b506102626103753660046118b9565b60036020526000908152604090205481565b34801561039357600080fd5b506102906103a23660046118b9565b610a2a565b3480156103b357600080fd5b506102626103c23660046118b9565b60056020526000908152604090205481565b3480156103e057600080fd5b506008546103f4906001600160a01b031681565b6040516001600160a01b0390911681526020016101bd565b34801561041857600080fd5b506102906104273660046118b9565b610b14565b34801561043857600080fd5b506006546103f4906001600160a01b031681565b34801561045857600080fd5b506101db610b68565b34801561046d57600080fd5b506101b161047c366004611586565b610b75565b34801561048d57600080fd5b5061023361049c366004611956565b63bc197c8160e01b95945050505050565b3480156104b957600080fd5b506007546103f4906001600160a01b031681565b3480156104d957600080fd5b506102906104e8366004611a04565b610b8b565b3480156104f957600080fd5b50610262610508366004611a7b565b600460209081526000928352604080842090915290825290205481565b34801561053157600080fd5b50610233610540366004611ab4565b63f23a6e6160e01b95945050505050565b34801561055d57600080fd5b5061029061056c3660046118b9565b610dcf565b34801561057d57600080fd5b506101db61058c366004611b1d565b610e4d565b60006001600160e01b03198216630271189760e51b14806105c257506301ffc9a760e01b6001600160e01b03198316145b92915050565b600080546105d590611ba8565b80601f016020809104026020016040519081016040528092919081815260200182805461060190611ba8565b801561064e5780601f106106235761010080835404028352916020019161064e565b820191906000526020600020905b81548152906001019060200180831161063157829003601f168201915b505050505081565b3360008181526004602090815260408083206001600160a01b038716808552925280832085905551919290917f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925906106b19086815260200190565b60405180910390a350600192915050565b6106d8336000356001600160e01b031916610ed5565b6106fd5760405162461bcd60e51b81526004016106f490611be2565b60405180910390fd5b6107078282610f7f565b8215610721576107216001600160a01b0385168685610fe9565b816001600160a01b0316846001600160a01b0316866001600160a01b03167fe0c82280a1164680e0cf43be7db4c4c9f985423623ad7a544fb76c772bdc60438685604051610779929190918252602082015260400190565b60405180910390a45050505050565b60606107a0336000356001600160e01b031916610ed5565b6107bc5760405162461bcd60e51b81526004016106f490611be2565b858067ffffffffffffffff8111156107d6576107d66115b2565b60405190808252806020026020018201604052801561080957816020015b60608152602001906001900390816107f45790505b50915060005b818110156108f3576108c587878381811061082c5761082c611c08565b905060200281019061083e9190611c1e565b8080601f01602080910402602001604051908101604052809392919081815260200183838082843760009201919091525089925088915085905081811061088757610887611c08565b905060200201358b8b858181106108a0576108a0611c08565b90506020020160208101906108b591906118b9565b6001600160a01b03169190611070565b8382815181106108d7576108d7611c08565b6020026020010181905250806108ec90611c7b565b905061080f565b50509695505050505050565b600061090b848461110d565b61091684848461118e565b90505b9392505050565b60007f000000000000000000000000000000000000000000000000000000000000000046146109565761095161126e565b905090565b507f000000000000000000000000000000000000000000000000000000000000000090565b610991336000356001600160e01b031916610ed5565b6109ad5760405162461bcd60e51b81526004016106f490611be2565b82156109c8576109c86001600160a01b038516863086611308565b6109d282826113a4565b816001600160a01b0316846001600160a01b0316866001600160a01b03167fea00f88768a86184a6e515238a549c171769fe7460a011d6fd0bcd48ca078ea48685604051610779929190918252602082015260400190565b6006546001600160a01b0316331480610abf575060075460405163b700961360e01b81526001600160a01b039091169063b700961390610a7e90339030906001600160e01b03196000351690600401611c94565b602060405180830381865afa158015610a9b573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610abf9190611cc1565b610ac857600080fd5b600780546001600160a01b0319166001600160a01b03831690811790915560405133907fa3396fd7f6e0a21b50e5089d2da70d5ac0a3bbbd1f617a93f134b7638998019890600090a350565b610b2a336000356001600160e01b031916610ed5565b610b465760405162461bcd60e51b81526004016106f490611be2565b600880546001600160a01b0319166001600160a01b0392909216919091179055565b600180546105d590611ba8565b6000610b81338461110d565b61091983836113f6565b42841015610bdb5760405162461bcd60e51b815260206004820152601760248201527f5045524d49545f444541444c494e455f4558504952454400000000000000000060448201526064016106f4565b60006001610be7610920565b6001600160a01b038a811660008181526005602090815260409182902080546001810190915582517f6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c98184015280840194909452938d166060840152608083018c905260a083019390935260c08083018b90528151808403909101815260e08301909152805192019190912061190160f01b6101008301526101028201929092526101228101919091526101420160408051601f198184030181528282528051602091820120600084529083018083525260ff871690820152606081018590526080810184905260a0016020604051602081039080840390855afa158015610cf3573d6000803e3d6000fd5b5050604051601f1901519150506001600160a01b03811615801590610d295750876001600160a01b0316816001600160a01b0316145b610d665760405162461bcd60e51b815260206004820152600e60248201526d24a72b20a624a22fa9a4a3a722a960911b60448201526064016106f4565b6001600160a01b0390811660009081526004602090815260408083208a8516808552908352928190208990555188815291928a16917f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925910160405180910390a350505050505050565b610de5336000356001600160e01b031916610ed5565b610e015760405162461bcd60e51b81526004016106f490611be2565b600680546001600160a01b0319166001600160a01b03831690811790915560405133907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e090600090a350565b6060610e65336000356001600160e01b031916610ed5565b610e815760405162461bcd60e51b81526004016106f490611be2565b610ecc84848080601f01602080910402602001604051908101604052809392919081815260200183838082843760009201919091525050506001600160a01b03881691905084611070565b95945050505050565b6007546000906001600160a01b03168015801590610f5f575060405163b700961360e01b81526001600160a01b0382169063b700961390610f1e90879030908890600401611c94565b602060405180830381865afa158015610f3b573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610f5f9190611cc1565b80610f7757506006546001600160a01b038581169116145b949350505050565b6001600160a01b03821660009081526003602052604081208054839290610fa7908490611ce3565b90915550506002805482900390556040518181526000906001600160a01b03841690600080516020611dc5833981519152906020015b60405180910390a35050565b600060405163a9059cbb60e01b81526001600160a01b0384166004820152826024820152602060006044836000895af13d15601f3d116001600051141617169150508061106a5760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b60448201526064016106f4565b50505050565b6060814710156110955760405163cd78605960e01b81523060048201526024016106f4565b600080856001600160a01b031684866040516110b19190611cf6565b60006040518083038185875af1925050503d80600081146110ee576040519150601f19603f3d011682016040523d82523d6000602084013e6110f3565b606091505b509150915061110386838361145c565b9695505050505050565b6008546001600160a01b03161561118a57600854604051630abd626b60e41b81526001600160a01b03848116600483015283811660248301523360448301529091169063abd626b09060640160006040518083038186803b15801561117157600080fd5b505afa158015611185573d6000803e3d6000fd5b505050505b5050565b6001600160a01b038316600090815260046020908152604080832033845290915281205460001981146111ea576111c58382611ce3565b6001600160a01b03861660009081526004602090815260408083203384529091529020555b6001600160a01b03851660009081526003602052604081208054859290611212908490611ce3565b90915550506001600160a01b0380851660008181526003602052604090819020805487019055519091871690600080516020611dc58339815191529061125b9087815260200190565b60405180910390a3506001949350505050565b60007f8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f60006040516112a09190611d12565b6040805191829003822060208301939093528101919091527fc89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc660608201524660808201523060a082015260c00160405160208183030381529060405280519060200120905090565b60006040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b03841660248201528260448201526020600060648360008a5af13d15601f3d116001600051141617169150508061139d5760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b60448201526064016106f4565b5050505050565b80600260008282546113b69190611db1565b90915550506001600160a01b038216600081815260036020908152604080832080548601905551848152600080516020611dc58339815191529101610fdd565b33600090815260036020526040812080548391908390611417908490611ce3565b90915550506001600160a01b03831660008181526003602052604090819020805485019055513390600080516020611dc5833981519152906106b19086815260200190565b6060826114715761146c826114b8565b610919565b815115801561148857506001600160a01b0384163b155b156114b157604051639996b31560e01b81526001600160a01b03851660048201526024016106f4565b5080610919565b8051156114c85780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b50565b6000602082840312156114f657600080fd5b81356001600160e01b03198116811461091957600080fd5b60005b83811015611529578181015183820152602001611511565b50506000910152565b6000815180845261154a81602086016020860161150e565b601f01601f19169290920160200192915050565b6020815260006109196020830184611532565b6001600160a01b03811681146114e157600080fd5b6000806040838503121561159957600080fd5b82356115a481611571565b946020939093013593505050565b634e487b7160e01b600052604160045260246000fd5b604051601f8201601f1916810167ffffffffffffffff811182821017156115f1576115f16115b2565b604052919050565b600082601f83011261160a57600080fd5b813567ffffffffffffffff811115611624576116246115b2565b611637601f8201601f19166020016115c8565b81815284602083860101111561164c57600080fd5b816020850160208301376000918101602001919091529392505050565b6000806000806080858703121561167f57600080fd5b843561168a81611571565b9350602085013561169a81611571565b925060408501359150606085013567ffffffffffffffff8111156116bd57600080fd5b6116c9878288016115f9565b91505092959194509250565b600080600080600060a086880312156116ed57600080fd5b85356116f881611571565b9450602086013561170881611571565b935060408601359250606086013561171f81611571565b949793965091946080013592915050565b60008083601f84011261174257600080fd5b50813567ffffffffffffffff81111561175a57600080fd5b6020830191508360208260051b850101111561177557600080fd5b9250929050565b6000806000806000806060878903121561179557600080fd5b863567ffffffffffffffff808211156117ad57600080fd5b6117b98a838b01611730565b909850965060208901359150808211156117d257600080fd5b6117de8a838b01611730565b909650945060408901359150808211156117f757600080fd5b5061180489828a01611730565b979a9699509497509295939492505050565b6000602080830181845280855180835260408601915060408160051b870101925083870160005b8281101561186b57603f19888603018452611859858351611532565b9450928501929085019060010161183d565b5092979650505050505050565b60008060006060848603121561188d57600080fd5b833561189881611571565b925060208401356118a881611571565b929592945050506040919091013590565b6000602082840312156118cb57600080fd5b813561091981611571565b600082601f8301126118e757600080fd5b8135602067ffffffffffffffff821115611903576119036115b2565b8160051b6119128282016115c8565b928352848101820192828101908785111561192c57600080fd5b83870192505b8483101561194b57823582529183019190830190611932565b979650505050505050565b600080600080600060a0868803121561196e57600080fd5b853561197981611571565b9450602086013561198981611571565b9350604086013567ffffffffffffffff808211156119a657600080fd5b6119b289838a016118d6565b945060608801359150808211156119c857600080fd5b6119d489838a016118d6565b935060808801359150808211156119ea57600080fd5b506119f7888289016115f9565b9150509295509295909350565b600080600080600080600060e0888a031215611a1f57600080fd5b8735611a2a81611571565b96506020880135611a3a81611571565b95506040880135945060608801359350608088013560ff81168114611a5e57600080fd5b9699959850939692959460a0840135945060c09093013592915050565b60008060408385031215611a8e57600080fd5b8235611a9981611571565b91506020830135611aa981611571565b809150509250929050565b600080600080600060a08688031215611acc57600080fd5b8535611ad781611571565b94506020860135611ae781611571565b93506040860135925060608601359150608086013567ffffffffffffffff811115611b1157600080fd5b6119f7888289016115f9565b60008060008060608587031215611b3357600080fd5b8435611b3e81611571565b9350602085013567ffffffffffffffff80821115611b5b57600080fd5b818701915087601f830112611b6f57600080fd5b813581811115611b7e57600080fd5b886020828501011115611b9057600080fd5b95986020929092019750949560400135945092505050565b600181811c90821680611bbc57607f821691505b602082108103611bdc57634e487b7160e01b600052602260045260246000fd5b50919050565b6020808252600c908201526b15539055551213d49256915160a21b604082015260600190565b634e487b7160e01b600052603260045260246000fd5b6000808335601e19843603018112611c3557600080fd5b83018035915067ffffffffffffffff821115611c5057600080fd5b60200191503681900382131561177557600080fd5b634e487b7160e01b600052601160045260246000fd5b600060018201611c8d57611c8d611c65565b5060010190565b6001600160a01b0393841681529190921660208201526001600160e01b0319909116604082015260600190565b600060208284031215611cd357600080fd5b8151801515811461091957600080fd5b818103818111156105c2576105c2611c65565b60008251611d0881846020870161150e565b9190910192915050565b600080835481600182811c915080831680611d2e57607f831692505b60208084108203611d4d57634e487b7160e01b86526022600452602486fd5b818015611d615760018114611d7657611da3565b60ff1986168952841515850289019650611da3565b60008a81526020902060005b86811015611d9b5781548b820152908501908301611d82565b505084890196505b509498975050505050505050565b808201808211156105c2576105c2611c6556feddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa2646970667358221220de0e50aca3a032802c7c415f6c52454b3ad8c884932f481b5836056ca11d843d64736f6c63430008150033"
);

#[async_trait]
impl Actionable for BoringVaultBlock {
    async fn to_actions(&self, vrm: &ViewRequestManager) -> Result<Vec<Box<dyn AdminAction>>> {
        let mut actions: Vec<Box<dyn AdminAction>> = Vec::new();
        // TODO make RPC calls checking if boring vault is deployed, if not add deploy action
        // then we would error here if the name, symbol, and decimal were not defined
        // Check if roles auth deployed
        // if yes, make sure roles are correct
        // if no, configure all roles

        // Check if boring vault is deployed.
        if let Some(boring_vault) = self.boring_vault_address {
            if vrm.request_code(boring_vault).await?.len() == 0 {
                // Boring vault is not deployed
                if self.boring_vault_name.is_none()
                    || self.boring_vault_symbol.is_none()
                    || self.boring_vault_decimals.is_none()
                {
                    return Err(eyre!(
                        "Deploying boring vault but missing name, symbol, or decimals"
                    ));
                }
            }
            let name = match self.boring_vault.as_ref().unwrap() {
                AddressOrContractName::ContractName(name) => name,
                AddressOrContractName::Address(_) => {
                    return Err(eyre!(
                        "BoringVaultBlock: Deploying boring vault but no name provided"
                    ));
                }
            };

            let constructor_args = Bytes::from(
                BoringVault::constructorCall::new((
                    Address::ZERO,
                    self.boring_vault_name.as_ref().unwrap().clone(),
                    self.boring_vault_symbol.as_ref().unwrap().clone(),
                    self.boring_vault_decimals.unwrap(),
                ))
                .abi_encode(),
            );

            let deploy_borign_vault_action = DeployContract::new(
                self.deployer.unwrap(),
                name.to_string(),
                BORING_VAULT_BYTECODE,
                constructor_args,
                U256::ZERO,
                0,
                CallerType::EOA(self.executor.unwrap()),
            );

            actions.push(Box::new(deploy_borign_vault_action));
        }
        Ok(actions)
    }

    // TODO current logic is blocking when waiting for address resolution, but
    // this can be refactored to concurrently get values from the cache
    async fn resolve_and_contribute(
        &mut self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<()> {
        if let Some(deployer) = &self.deployer {
            cache
                .set(
                    "deployer",
                    CacheValue::Address(*deployer),
                    "boring_vault_block",
                )
                .await?;
        } else {
            // Read the value from the cache.
            let result = cache.get("deployer", "boring_vault_block").await?;
            match result {
                CacheValue::Address(addr) => self.deployer = Some(addr),
                _ => return Err(eyre!("BoringVaultBlock: Cache deployer is not an address")),
            }
        }

        if let Some(boring_vault) = &self.boring_vault {
            match boring_vault {
                AddressOrContractName::Address(addr) => {
                    if vrm.request_code(*addr).await?.len() == 0 {
                        return Err(eyre!(
                            "BoringVaultBlock: Contract name must be specified to deploy boring vault"
                        ));
                    }
                    self.boring_vault_address = Some(*addr);
                    cache
                        .set(
                            "boring_vault",
                            CacheValue::Address(*addr),
                            "boring_vault_block",
                        )
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.boring_vault_address = Some(addr);
                        cache
                            .set(
                                "boring_vault",
                                CacheValue::Address(addr),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("boring_vault", "boring_vault_block").await?;
            match result {
                CacheValue::Address(addr) => self.boring_vault_address = Some(addr),
                _ => {
                    return Err(eyre!(
                        "BoringVaultBlock: Cache boring_vault is not an address"
                    ));
                }
            }
        }

        if let Some(roles_authority) = &self.roles_authority {
            match roles_authority {
                AddressOrContractName::Address(addr) => {
                    self.roles_authority_address = Some(*addr);
                    cache
                        .set(
                            "roles_authority",
                            CacheValue::Address(*addr),
                            "boring_vault_block",
                        )
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.roles_authority_address = Some(addr);
                        cache
                            .set(
                                "roles_authority",
                                CacheValue::Address(addr),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("roles_authority", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.roles_authority_address = Some(addr),
                    _ => {
                        return Err(eyre!(
                            "BoringVaultBlock: Cache roles_authority is not an address"
                        ));
                    }
                }
            } else {
                // Try to query the roles authority from the boring vault.
                if let Some(boring_vault) = &self.boring_vault_address {
                    let calldata = Bytes::from(Auth::authorityCall::new(()).abi_encode());
                    let result = vrm.request(*boring_vault, calldata).await;
                    if let Ok(res) = result {
                        let data = Auth::authorityCall::abi_decode_returns(&res, true)?;
                        self.roles_authority_address = Some(data.authority);
                        cache
                            .set(
                                "roles_authority",
                                CacheValue::Address(data.authority),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        }

        if let Some(decimals) = self.boring_vault_decimals {
            cache
                .set(
                    "boring_vault_decimals",
                    CacheValue::U8(decimals),
                    "boring_vault_block",
                )
                .await?;
        } else {
            // Try to query decimals from the boring vault.
            if let Some(boring_vault) = &self.boring_vault_address {
                let calldata = Bytes::from(BoringVault::decimalsCall::new(()).abi_encode());
                let result = vrm.request(*boring_vault, calldata).await;
                if let Ok(res) = result {
                    let data = BoringVault::decimalsCall::abi_decode_returns(&res, true)?;
                    self.boring_vault_decimals = Some(data._0);
                    cache
                        .set(
                            "boring_vault_decimals",
                            CacheValue::U8(data._0),
                            "boring_vault_block",
                        )
                        .await?;
                } // else leave boring_vault_decimals as None
            }
        }

        if let Some(hook) = &self.hook {
            match hook {
                AddressOrContractName::Address(addr) => {
                    self.hook_address = Some(*addr);
                    cache
                        .set("hook", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.hook_address = Some(addr);
                        cache
                            .set("hook", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("hook", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.hook_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache hook is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: hook address not defined locally or in cache");
            }
        }

        if let Some(manager) = &self.manager {
            match manager {
                AddressOrContractName::Address(addr) => {
                    self.manager_address = Some(*addr);
                    cache
                        .set("manager", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.manager_address = Some(addr);
                        cache
                            .set("manager", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("manager", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.manager_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache manager is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: manager address not defined locally or in cache");
            }
        }

        if let Some(teller) = &self.teller {
            match teller {
                AddressOrContractName::Address(addr) => {
                    self.teller_address = Some(*addr);
                    cache
                        .set("teller", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.teller_address = Some(addr);
                        cache
                            .set("teller", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("teller", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.teller_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache teller is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: teller address not defined locally or in cache");
            }
        }

        if let Some(executor) = self.executor {
            cache
                .set("executor", CacheValue::Address(executor), "global_block")
                .await?;
        } else {
            // Try reading executor from cache.
            let result = cache.get("executor", "boring_vault_block").await?;
            match result {
                CacheValue::Address(addr) => self.executor = Some(addr),
                _ => {
                    return Err(eyre!(
                        "BoringVaultBlock: executor not defined locally or in cache"
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_manager::block_manager::BlockManager;
    use alloy::primitives::address;
    use serde_json::json;

    const RPC_URL: &str = "https://eth.llamarpc.com";

    async fn setup_block_manager(json: serde_json::Value) -> BlockManager {
        let mut manager = BlockManager::new(RPC_URL.to_string()).await.unwrap();
        manager.create_blocks_from_json_value(json).unwrap();
        manager
    }

    #[tokio::test]
    async fn test_scenario_a_full_config() {
        let json = json!([
            {
                "BoringVault": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
                    "roles_authority": "0x1111111111111111111111111111111111111111",
                    "boring_vault_name": "Test Vault",
                    "boring_vault_symbol": "TV",
                    "boring_vault_decimals": 18,
                    "hook": "0x2222222222222222222222222222222222222222",
                    "manager": "0x3333333333333333333333333333333333333333",
                    "teller": "0x4444444444444444444444444444444444444444",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x1111111111111111111111111111111111111111"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert_eq!(
            cache.get_immediate("hook").await.unwrap(),
            CacheValue::Address(address!("0x2222222222222222222222222222222222222222"))
        );
        assert_eq!(
            cache.get_immediate("manager").await.unwrap(),
            CacheValue::Address(address!("0x3333333333333333333333333333333333333333"))
        );
        assert_eq!(
            cache.get_immediate("teller").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );
    }

    #[tokio::test]
    async fn test_scenario_b_minimal_config() {
        let json = json!([
            {
                "BoringVault": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );

        // Verify other values are not set
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x485Bde66Bb668a51f2372E34e45B1c6226798122"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert!(cache.get_immediate("hook").await.is_err());
        assert!(cache.get_immediate("manager").await.is_err());
        assert!(cache.get_immediate("teller").await.is_err());
    }

    #[tokio::test]
    async fn test_scenario_c_global_and_boring_vault() {
        let json = json!([
            {
                "Global": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "Test Boring Vault V0.100",
                    "roles_authority": "0x4444444444444444444444444444444444444444",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            },
            {
                "BoringVault": {
                    "boring_vault_name": "Test Vault",
                    "boring_vault_symbol": "TV",
                    "boring_vault_decimals": 18
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        // Verify values from Global block
        assert_eq!(
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        let expected_boring_vault = derive_contract_address(
            "Test Boring Vault V0.100",
            address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"),
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(expected_boring_vault)
        );
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );

        // Verify values from BoringVault block
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
    }
}
