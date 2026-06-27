import api from './api';

export interface ApiToken {
  id: string;
  name: string;
  last_used_at: string | null;
  expires_at: string | null;
  created_at: string;
  webdav_enabled: boolean;
  webdav_read_only: boolean;
  webdav_root_folder_id: string | null;
}

export interface CreateApiTokenRequest {
  name: string;
  expires_in_days?: number;
  webdav_enabled?: boolean;
  webdav_read_only?: boolean;
  webdav_root_folder_id?: string | null;
}

export interface CreateApiTokenResponse {
  token: {
    id: string;
    name: string;
    token: string; // Only shown once
    expires_at: string | null;
    created_at: string;
    webdav_enabled: boolean;
    webdav_read_only: boolean;
    webdav_root_folder_id: string | null;
  };
}

export interface ListApiTokensResponse {
  tokens: ApiToken[];
}

export const apiTokenService = {
  async listTokens(): Promise<ApiToken[]> {
    const response = await api.get<ListApiTokensResponse>('/api/tokens');
    return response.data.tokens;
  },

  async createToken(
    request: CreateApiTokenRequest
  ): Promise<CreateApiTokenResponse> {
    const response = await api.post<CreateApiTokenResponse>(
      '/api/tokens',
      request
    );
    return response.data;
  },

  async deleteToken(tokenId: string): Promise<void> {
    await api.delete(`/api/tokens/${tokenId}`);
  },
};
