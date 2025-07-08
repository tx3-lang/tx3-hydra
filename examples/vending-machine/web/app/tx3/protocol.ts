// This file is auto-generated.

import { Client as TRPClient, type ClientOptions, type TxEnvelope } from 'tx3-sdk/trp';

export const DEFAULT_TRP_ENDPOINT = "http://localhost:3000/trp";

export const DEFAULT_HEADERS = {
};

export const DEFAULT_ENV_ARGS = {
};

export type TransferParams = {
    quantity: number;
    receiver: string;
    sender: string;
}

export const TRANSFER_IR = {
    bytecode: "0d03000106736f757263650d0206736f757263650d010673656e646572050e010c0100000d01087175616e74697479020d03000000020d0108726563656976657205000c0100000d01087175616e74697479020d010673656e64657205000e020e020f010d0206736f757263650d010673656e646572050e010c0100000d01087175616e74697479020d03000c0100000d01087175616e74697479020d03000000000000",
    encoding: "hex",
    version: "v1alpha6",
};

export class Client {
    readonly #client: TRPClient;

    constructor(options: ClientOptions) {
        this.#client = new TRPClient(options);
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
