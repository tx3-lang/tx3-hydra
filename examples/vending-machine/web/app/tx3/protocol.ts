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
}

export const MINT_FROM_SCRIPT_IR = {
    bytecode: "0d03000106736f757263650d0206736f757263650d01066d696e746572050d03000000010d01066d696e74657205000e020e010f010d0206736f757263650d01066d696e746572050d03000c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e74697479020d0300010c01041cbd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb77707034142430d01087175616e7469747902030000010e706c757475735f7769746e657373020776657273696f6e05060673637269707404125101010023259800a518a4d136564004ae69010d01066d696e746572050c01000005fc80969800000000",
    encoding: "hex",
    version: "v1alpha6",
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
}

// Create a default client instance
export const protocol = new Client({
    endpoint: DEFAULT_TRP_ENDPOINT,
    headers: DEFAULT_HEADERS,
    envArgs: DEFAULT_ENV_ARGS,
});
