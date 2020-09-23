import React from "react";
import detectEthereumProvider from '@metamask/detect-provider';
import {Transaction} from 'ethereumjs-tx';
import {keccak256} from "ethereum-cryptography/keccak";
import {sha256} from "ethereum-cryptography/sha256";
import {secp256k1} from 'ethereum-cryptography/secp256k1';
import {ec as EC} from 'elliptic';

export let account = null;

class Web3Connect extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            accounts: null,
            account: null
        };
    }

    async load() {
        let provider = await detectEthereumProvider();
        const {handleAccountChange} = this.props;
        if (provider) {
            provider.on('accountsChanged', (accounts) => {
                account = new Account(accounts[0], provider);
                this.setState({accounts, account});
                console.log('ss', handleAccountChange, accounts);
                if (handleAccountChange) {
                    handleAccountChange(account)
                }
            });
            const accounts = await provider.request({method: 'eth_requestAccounts'});
            account = new Account(accounts[0], provider);
            this.setState({accounts, account});
            if (handleAccountChange) {
                handleAccountChange(account)
            }

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
        this.ec = new EC('secp256k1');

        if ('sessionKey' in sessionStorage) {
            this.sesionKey = this.ec.keyFromPrivate(sessionStorage['sessionKey'])
        }
        else {
            this.sesionKey = this.ec.genKeyPair();
        }
        sessionStorage['sessionKey'] = this.sesionKey.getPrivate().toString('hex');

        window.tx_bk = this;
    }

    pubToAddress(key) {
        const sessionKey = Buffer.from(key.encode());
        const bytes = keccak256(sessionKey.slice(1)).subarray(12);

        return Array.from(bytes).map((ch) => ('00' + ch.toString(16)).slice(-2)).join('')
    }

    sessionAddress() {
        const sessionKey = Buffer.from(this.sesionKey.getPublic().encode());

        const bytes = keccak256(sessionKey.slice(1)).subarray(12);

        return Array.from(bytes).map((ch) => ('00' + ch.toString(16)).slice(-2)).join('')
    }

    validateTicket(contract, votingId, sender, signature) {
        const head = keccak256("SgxVotingTicket(address,bytes,address)");
        let utf8 = new TextEncoder('utf-8');
        let vid = utf8.encode(votingId)

        let bx = Buffer.concat([
            head,
            Buffer.from(this.hex2a(contract)),
            Buffer.from(vid),
            Buffer.from(this.hex2a(sender))
        ]);
        let h = keccak256(bx);
        //let signature_bytes = this.hex2a(signature);
        let j = parseInt(signature.slice(-2), 2);
        let r = signature.slice(0, 64);
        let s = signature.slice(64, 64*2);
        const managerPubKey = this.ec.recoverPubKey(h, {r, s}, j);
        const resolvedAddress = this.pubToAddress(managerPubKey)
        return {managerPubKey, resolvedAddress};
    }

    hex2a(hex) {
        if (hex.startsWith('0x')) {
            hex = hex.substring(2);
        }
        const nBytes = hex.length / 2;
        let bytes = new Uint8Array(nBytes);
        for (let i=0; i<nBytes; ++i) {
            bytes[i] = parseInt(hex.substring(i*2, i*2+2), 16)
        }
        return bytes
    }

    a2hex(a) {
        return Uint8Array.from(a).map((ch) => ('00' + ch.toString(16)).slice(-2)).join('')
    }

    async signRegistration(contract, votingId, managerAddress) {
        const sessionKey = this.sesionKey.getPublic();
        const msg = `\nSgxRegister\nContract: ${contract} ${votingId}\nAddress: ${managerAddress}\nSession: ${this.sessionAddress()}`;
        //const hash = this.provider.hashMessage(msg);
        //console.log("hash", hash);

        const accountId = this.accountId;
        const signature = await this.provider.request({
            method: "personal_sign",
            params: [msg, this.accountId]
        });

        return {sessionKey, accountId, signature};
    }

    sharedSecret(pubKey) {
        const d = pubKey.mul(this.sesionKey.getPrivate());
        const ss_bytes = [ 0x2 | (d.getY().isOdd() ? 1 : 0)].concat(d.getX().toArray('be', 32));
        // .toString(16, 64)
        return sha256(ss_bytes);
    }

    async encryptVote(decision, ticket, mgrPubKey) {
        const key_bytes = this.sharedSecret(mgrPubKey);
        let key = await crypto.subtle.importKey('raw', key_bytes, 'AES-GCM', false, ['encrypt']);
        const bytes = new Uint8Array([decision, 0, 0, 0]);
        const iv = window.crypto.getRandomValues(new Uint8Array(12));
        const encryptedMesssage = await window.crypto.subtle.encrypt(
            {
                name: "AES-GCM",
                iv: iv
            },
            key,
            bytes
        );
        console.log('message', bytes, 'iv', iv, 'enc', encryptedMesssage);

        return [ ... iv, ... new Uint8Array(encryptedMesssage)];
    }

    async decryptVote(mgrPubKey, message) {
        message = Array.from(message);

        const key_bytes = this.sharedSecret(mgrPubKey);

        let key = await crypto.subtle.importKey('raw', key_bytes, 'AES-GCM', false, ['decrypt']);

        const iv = new Uint8Array(message.slice(null, 12));
        const encrypted = new Uint8Array(message.slice(12));
        console.log('message', message, 'iv', iv, 'enc', encrypted);
        return await window.crypto.subtle.decrypt(
            {
                name: "AES-GCM",
                iv: iv
            },
            key,
            encrypted
        );
    }

    genPairs() {
        const k1 = this.ec.genKeyPair();
        const k2 = this.ec.genKeyPair();

        //
        const sa1 = k2.getPublic().mul(k1.getPrivate());
        const ss_bytes = [ 0x2 | (sa1.getY().isOdd() ? 1 : 0)].concat(sa1.getX().toArray('be'));
        const ssx = sha256(ss_bytes).toString('hex');

        //

        const s1 = k1.derive(k2.getPublic()).toArray('be');
        const s2 = k2.derive(k1.getPublic()).toArray('be');


        const sa2 = k1.getPublic().mul(k2.getPrivate());

        return [
            {
                secret: k1.getPrivate().toString(16, 64),
                public: k1.getPublic().encode('hex'),
                address: this.pubToAddress(k1.getPublic()),
                shared: this.a2hex(s1),
                sa: sa1,
                ssx,
                x: [ 0x2 | (sa1.getY().isOdd() ? 1 : 0)].concat(sa1.getX().toArray('be')),
            },
            {
                secret: k2.getPrivate().toString(16, 64),
                public: k2.getPublic().encode('hex'),
                address: this.pubToAddress(k2.getPublic()),
                shared: this.a2hex(s2),
                sa: sa2,
                x: sa1.getX(),
            }
        ]
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