import React from 'react';
import PropTypes from 'prop-types';
import Avatar from '@material-ui/core/Avatar';
import Button from '@material-ui/core/Button';
import CssBaseline from '@material-ui/core/CssBaseline';
import LockIcon from '@material-ui/icons/LockOutlined';
import Paper from '@material-ui/core/Paper';
import Typography from '@material-ui/core/Typography';
import withStyles from '@material-ui/core/styles/withStyles';
import { NavigationBarSimple } from '../shared';
import { withRouter } from 'react-router-dom';

import { styles } from './styles';

class LoginComponent extends React.Component {

    onSubmit = (e) => {
        e.preventDefault();
        this.props.history.push({ pathname: '/Dashboard' });
    }

    render() {
        const { classes } = this.props;
        return (
            <React.Fragment>
                <CssBaseline />
                <div className={classes.main}>
                    {NavigationBarSimple(this.props)}
                    <div className={classes.layout}>
                        <Paper className={classes.paper}>
                            {this.renderSingInHeader(classes)}
                            {this.renderForm(classes)}
                        </Paper>
                    </div>
                </div>
            </React.Fragment>
        );
    }

    renderSingInHeader() {
        const { classes } = this.props;
        return (
            <div>
                <Typography component="h1" variant="h5">
                    Jormungandr log viewer
                </Typography>
            </div>
        )
    }

    renderForm(classes) {
        return (
            <form className={classes.form} onSubmit={this.onSubmit.bind(this)}>
                {this.renderButton()}
            </form>);
    }

    renderButton() {
        const { classes } = this.props;
        return (<Button
            type="submit"
            fullWidth
            variant="contained"
            color={this.props.isLoading ? "secondary" : "primary"}
            disabled={this.props.isLoading}
            className={classes.submit}
        > Enter</Button>)
    }
}

LoginComponent.propTypes = {
    classes: PropTypes.object.isRequired,
};


export default withStyles(styles)(withRouter(LoginComponent))