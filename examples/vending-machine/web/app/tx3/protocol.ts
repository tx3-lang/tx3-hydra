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
    bytecode: "12000106736f7572636501010d0673656e6465720501110c0100000d087175616e746974790212000000000002010d0872656365697665720500010c0100000d087175616e7469747902010d0673656e64657205000111111006736f757263650c0100000d087175616e7469747902011201000000000000",
    encoding: "hex",
    version: "v1alpha5",
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
