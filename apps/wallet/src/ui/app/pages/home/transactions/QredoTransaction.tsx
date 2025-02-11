// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useOnScreen } from '@mysten/core';
import { IntentScope, fromB64 } from '@mysten/sui.js';
import { useMemo, useRef } from 'react';

import { toUtf8OrB64 } from '_src/shared/utils';
import { TxnIcon } from '_src/ui/app/components/transactions-card/TxnIcon';
import { useGetQredoTransaction } from '_src/ui/app/hooks/useGetQredoTransaction';
import { Text } from '_src/ui/app/shared/text';
export type QredoTransactionProps = {
    qredoID?: string;
    qredoTransactionID?: string;
};

export function QredoTransaction({
    qredoID,
    qredoTransactionID,
}: QredoTransactionProps) {
    const transactionElementRef = useRef<HTMLDivElement>(null);
    const { isIntersecting } = useOnScreen(transactionElementRef);
    const { data, isLoading, error } = useGetQredoTransaction({
        qredoID,
        qredoTransactionID,
        forceDisabled: !isIntersecting,
    });
    const messageWithIntent = useMemo(() => {
        if (data?.MessageWithIntent) {
            return fromB64(data.MessageWithIntent);
        }
        return null;
    }, [data?.MessageWithIntent]);
    const scope = messageWithIntent?.[0];
    const isSignMessage = scope === IntentScope.PersonalMessage;
    const transactionBytes = useMemo(
        () => messageWithIntent?.slice(3) || null,
        [messageWithIntent]
    );
    const messageToSign =
        useMemo(
            () => transactionBytes && toUtf8OrB64(transactionBytes),
            [transactionBytes]
        )?.message?.slice(0, 300) || null;
    return (
        <div
            ref={transactionElementRef}
            className="py-4 flex items-start gap-3"
        >
            <div>
                <TxnIcon
                    txnFailed={!!error}
                    variant={
                        isLoading
                            ? 'Loading'
                            : isSignMessage
                            ? 'PersonalMessage'
                            : 'Send'
                    }
                />
            </div>
            <div className="flex flex-col gap-1">
                {isLoading ? (
                    <>
                        <div className="bg-sui-lightest h-3 w-20 rounded" />
                        <div className="bg-sui-lightest h-3 w-16 rounded" />
                    </>
                ) : data ? (
                    <>
                        <div className="flex flex-nowrap gap-1 item-center">
                            <Text color="gray-90" weight="semibold">
                                {isSignMessage
                                    ? 'Sign personal message'
                                    : 'Transaction'}
                            </Text>
                            <Text color="gray-90" variant="bodySmall">
                                ({data.status})
                            </Text>
                        </div>
                        <Text color="gray-80" mono variant="bodySmall">
                            #{data.txID}
                        </Text>
                        {isSignMessage && messageToSign ? (
                            <div className="break-all">
                                <Text color="gray-80" weight="normal">
                                    {messageToSign}
                                </Text>
                            </div>
                        ) : null}
                    </>
                ) : (
                    <Text color="gray-80">
                        {(error as Error)?.message ||
                            'Something went wrong while fetching transaction details'}
                    </Text>
                )}
            </div>
        </div>
    );
}
