import fs from 'fs';
import path from 'path';

interface AdminCredentials {
  privateKey: string;
  address: string;
}


const ADMIN_CREDENTIAL_PATH = process.env["ADMIN_CREDENTIAL_PATH"] || "../chain/cardano"

export function getAdminCredentials(): AdminCredentials {
  try {
    const privateKeyfileContent = fs.readFileSync(`${ADMIN_CREDENTIAL_PATH}/user.sk`, 'utf-8');
    const privateKeyJsonContent = JSON.parse(privateKeyfileContent)
    const privateKey = privateKeyJsonContent["cborHex"]

    const address = fs.readFileSync(`${ADMIN_CREDENTIAL_PATH}/user.addr`, 'utf-8');

    return {
      privateKey,
      address
    };
  } catch (error) {
    console.error('Error loading admin credentials:', error);
    throw new Error('Failed to load admin credentials');
  }
}
