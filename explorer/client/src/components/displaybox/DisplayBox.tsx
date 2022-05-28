// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback, useEffect, useContext } from 'react';

import { NetworkContext } from '../../context';
import { ImageModClient } from '../../utils/imageModeratorClient';
import { processDisplayValue } from '../../utils/stringUtils';

import styles from './DisplayBox.module.css';

function DisplayBox({ display }: { display: string }) {
    const [hasDisplayLoaded, setHasDisplayLoaded] = useState(false);
    const [hasFailedToLoad, setHasFailedToLoad] = useState(false);

    const [hasImgBeenChecked, setHasImgBeenChecked] = useState(false);
    const [imgAllowState, setImgAllowState] = useState(false);
    const [network] = useContext(NetworkContext);

    const imageStyle = hasDisplayLoaded ? {} : { display: 'none' };
    const handleImageLoad = useCallback(
        () => setHasDisplayLoaded(true),
        [setHasDisplayLoaded]
    );

    useEffect(() => {
        setHasFailedToLoad(false);
        setHasImgBeenChecked(false);
        setImgAllowState(false);
        console.log(
            `start DisplayBox image content check, network: ${network}`
        );

        new ImageModClient(network)
            .checkImage(processDisplayValue(display))
            .then(({ ok }) => {
                //setHasImgBeenChecked(true);
                setImgAllowState(ok);
                console.log(
                    `image allow check for ${display} returned:  ${ok}`
                );
            })
            .catch((error) => {
                console.error('image content check threw', error);
                //setHasImgBeenChecked(true);
                // default to allow, so a broken img check service doesn't break NFT display
                setImgAllowState(true);
            })
            .finally(() => {
                setHasImgBeenChecked(true);
            });
    }, [display, network]);

    const handleImageFail = useCallback(
        (error) => {
            console.log(error);
            setHasDisplayLoaded(true);
            setHasFailedToLoad(true);
        },
        [setHasFailedToLoad]
    );

    console.log(
        `img allow state: ${imgAllowState}`,
        `checked state: ${imgAllowState}, has display loaded: ${hasDisplayLoaded}`
    );

    // if we've loaded the display image but the check hasn't returned, display a blurry version
    const shouldBlur = hasDisplayLoaded && !hasImgBeenChecked && !imgAllowState;
    if (shouldBlur)
        console.log(
            'SHOULD BLUR???',
            hasDisplayLoaded,
            hasImgBeenChecked,
            imgAllowState
        );

    const imgClass = shouldBlur ? styles.imageboxblur : styles.imagebox;
    console.log('img class', imgClass);

    const imgSrc = processDisplayValue(display);

    return (
        <div className={styles['display-container']}>
            {!hasDisplayLoaded ||
                (hasDisplayLoaded && !imgAllowState && (
                    <div className={styles.imagebox} id="pleaseWaitImage">
                        image loading...
                    </div>
                ))}
            {hasFailedToLoad ? (
                <div className={styles.imagebox} id="noImage">
                    No Image was Found
                </div>
            ) : (
                <img
                    id="loadedImage"
                    className={imgClass}
                    style={imageStyle}
                    alt="NFT"
                    // TODO - substitute auto-mod block image for src
                    src={imgSrc}
                    onLoad={handleImageLoad}
                    onError={handleImageFail}
                />
            )}
        </div>
    );
}

export default DisplayBox;
