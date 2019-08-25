import React from 'react';

import Button from '@material-ui/core/Button';
import Typography from '@material-ui/core/Typography';

function ChannelConfirmationPage(props) {
    return (<React.Fragment>
        <Typography variant="h5" gutterBottom>
            Node was registered.
        </Typography>
        <Typography variant="subtitle1">
            under address: {props.data.data.host}
        </Typography>
        <Button
            type="submit"
            variant="contained"
            color="primary"
            onClick={() => props.successAction()}
        >Ok
        </Button>

    </React.Fragment>);
}

export default ChannelConfirmationPage;