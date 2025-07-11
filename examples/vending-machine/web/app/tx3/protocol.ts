// This file is auto-generated.

import { Client as TRPClient, type ClientOptions, type TxEnvelope } from 'tx3-sdk/trp';

export const DEFAULT_TRP_ENDPOINT = "http://localhost:3000/trp";

export const DEFAULT_HEADERS = {
};

export const DEFAULT_ENV_ARGS = {
};

export type MintFromScriptParams = {
    minter: string;
    quantity: number;
    receiver: string;
}

export const MINT_FROM_SCRIPT_IR = {
    bytecode: "0d03000106736f757263650d0206736f757263650d01066d696e74657205000000020d0108726563656976657205000c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e74697479020d01066d696e74657205000e0210010d0206736f757263650d01066d696e7465720500000d0300010c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e7469747902030000010e706c757475735f7769746e657373020776657273696f6e05060673637269707404125101010023259800a518a4d136564004ae69010d020a636f6c6c61746572616c0d01066d696e7465720500000000",
    encoding: "hex",
    version: "v1alpha7",
};

export type TransferParams = {
    quantity: number;
    receiver: string;
    sender: string;
}

export const TRANSFER_IR = {
    bytecode: "0d03000106736f757263650d0206736f757263650d010673656e64657205000000020d0108726563656976657205000c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e74697479020d010673656e64657205000e020e0210010d0206736f757263650d010673656e6465720500000c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e74697479020d03000000000000",
    encoding: "hex",
    version: "v1alpha7",
};

export class Client {
    readonly #client: TRPClient;

    constructor(options: ClientOptions) {
        this.#client = new TRPClient(options);
    }

    async mintFromScriptTx(args: MintFromScriptParams): Promise<TxEnvelope> {
        return await this.#client.resolve({
            tir: MINT_FROM_SCRIPT_IR,
            args,
        });
    }
    async transferTx(args: TransferParams): Promise<TxEnvelope> {
        return await this.#client.resolve({
            tir: TRANSFER_IR,
            args,
        });
    }
}

// Create a default client instance
export const protocol = new Client({
    endpoint: DEFAULT_TRP_ENDPOINT,
    headers: DEFAULT_HEADERS,
    envArgs: DEFAULT_ENV_ARGS,
});
