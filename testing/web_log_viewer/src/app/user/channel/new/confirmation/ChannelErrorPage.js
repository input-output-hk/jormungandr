import React from 'react';
import PropTypes from 'prop-types';
import { withStyles } from '@material-ui/core/styles';
import Typography from '@material-ui/core/Typography';
import List from '@material-ui/core/List';
import ListItem from '@material-ui/core/ListItem';
import ListItemText from '@material-ui/core/ListItemText';
import Grid from '@material-ui/core/Grid';
import { styles } from '../styles'
import Button from '@material-ui/core/Button';
import errorImage from '../../../../../images/error.png'

function ChannelErrorPage(props) {
    return (<React.Fragment>
        <img src={errorImage} alt='error icon' />
        <Typography variant="h5" gutterBottom>
            Node was not registered.
        </Typography>
        <Typography variant="subtitle1">
            Cannot connect ot node with url:  {props.data.host}
        </Typography>
        <Typography variant="subtitle1">
            due to {props.error}. Please ensure above address is available
        </Typography>
        <Typography variant="subtitle1">
            <Button
                type="submit"
                variant="contained"
                color="primary"
                onClick={() => props.resetAction()}
            >Retry
        </Button>
        </Typography>

    </React.Fragment>);
}

export default withStyles(styles)(ChannelErrorPage);