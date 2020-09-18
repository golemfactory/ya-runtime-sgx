import React from "react";
import detectEthereumProvider from '@metamask/detect-provider';
import {Transaction} from 'ethereumjs-tx';

export let account = null;

class Web3Connect extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            accounts: null
        };
    }

    async load() {
        let provider = await detectEthereumProvider();
        if (provider) {
            provider.on('accountsChanged', (accounts) => this.setState({accounts}));
            const accounts = await provider.request({method: 'eth_requestAccounts'});
            this.setState({accounts});
            account = new Account(accounts[0], provider);
        }
    }

    componentDidMount() {
        this.load()
    }

    render() {
        const {accounts} = this.state;
        if (accounts) {
            return <div className="navbar-text"><b className="badge badge-info">web3: {accounts[0]}</b></div>;
        }
        else {
            return <div className="navbar-text"><b className="badge badge-danger">web3 not connected</b></div>;
        }
    }
}

class Account {
    constructor(accountId, provider) {
        this.accountId = accountId;
        this.provider = provider;
        window.tx_bk = this;
    }

    async signRegistration(contract, votingId, managerAddress) {
        const msg = `RegisterToVote\nContract: ${contract} ${votingId}\nAddress: ${managerAddress}`;
        //const hash = this.provider.hashMessage(msg);
        //console.log("hash", hash);

        const signature = await this.provider.request({
            method: "personal_sign",
            params: [msg, this.accountId]
        });
        console.log('signature', signature);
        const address = await this.provider.request({
            method: "personal_ecRecover",
            params: [msg, signature]
        });
        console.log('address', address);
        const pub_key = await this.provider.request({
            method: "eth_getEncryptionPublicKey",
            params: [this.accountId]
        });
        console.log('pub_key', pub_key);
        return signature;
    }

    async recover_pub_key(tx_hash) {
        const tx = await this.provider.request({  method: "eth_getTransactionByHash", params: [tx_hash] });
        const pubkey = new Transaction({
            nonce: tx.nonce,
            gasPrice: tx.gasPrice,
            gasLimit: tx.gas,
            to: tx.to,
            value: tx.value,
            data: tx.input,
            chainId: '4', // mainnet network ID is 1. or use web3.version.network to find out
            r: tx.r,
            s: tx.s,
            v: tx.v,
        }, {chain: 4}).getSenderPublicKey()
        return pubkey;
    }

}

export default Web3Connect;